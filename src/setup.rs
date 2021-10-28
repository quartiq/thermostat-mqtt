use log::info;

use crate::{
    adc::{Adc, AdcPins},
    dac::{Dac0Pins, Dac1Pins, Dacs, Pwms},
    leds::Leds,
};

use smoltcp_nal::smoltcp;
use smoltcp_nal::smoltcp::{
    iface::{InterfaceBuilder, Routes},
    wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address},
};

use stm32_eth::{
    hal::gpio::GpioExt,
    hal::hal::digital::v2::OutputPin,
    hal::rcc::RccExt,
    hal::time::{MegaHertz, U32Ext},
    {EthPins, PhyAddress, RingEntry, RxDescriptor, TxDescriptor},
};

use rtt_logger::RTTLogger;

const HSE: MegaHertz = MegaHertz(8);

type Eth = stm32_eth::Eth<'static, 'static>;

const SRC_MAC: [u8; 6] = [0x80, 0x1f, 0x12, 0x63, 0x84, 0x1a];

const NUM_TCP_SOCKETS: usize = 2;
const NUM_UDP_SOCKETS: usize = 0;
const NUM_SOCKETS: usize = NUM_UDP_SOCKETS + NUM_TCP_SOCKETS;

pub struct NetStorage {
    pub ip_addrs: [smoltcp::wire::IpCidr; 1],
    pub sockets: [Option<smoltcp::socket::SocketSetItem<'static>>; NUM_SOCKETS],
    pub tcp_socket_storage: [TcpSocketStorage; NUM_TCP_SOCKETS],
    pub udp_socket_storage: [UdpSocketStorage; NUM_UDP_SOCKETS],
    pub neighbor_cache: [Option<(smoltcp::wire::IpAddress, smoltcp::iface::Neighbor)>; 4],
    pub routes_cache: [Option<(smoltcp::wire::IpCidr, smoltcp::iface::Route)>; 4],
}

#[derive(Copy, Clone)]
pub struct UdpSocketStorage {
    rx_storage: [u8; 128],
    tx_storage: [u8; 128],
    tx_metadata: [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
    rx_metadata: [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
}

impl UdpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; 128],
            tx_storage: [0; 128],
            tx_metadata: [smoltcp::storage::PacketMetadata::<smoltcp::wire::IpEndpoint>::EMPTY; 10],
            rx_metadata: [smoltcp::storage::PacketMetadata::<smoltcp::wire::IpEndpoint>::EMPTY; 10],
        }
    }
}

#[derive(Copy, Clone)]
pub struct TcpSocketStorage {
    rx_storage: [u8; 128],
    tx_storage: [u8; 128],
}

impl TcpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; 128],
            tx_storage: [0; 128],
        }
    }
}

impl Default for NetStorage {
    fn default() -> Self {
        NetStorage {
            ip_addrs: [IpCidr::new(
                IpAddress::from(Ipv4Address::new(10, 42, 0, 18)),
                24,
            )],
            neighbor_cache: [None; 4],
            routes_cache: [None; 4],
            sockets: [None, None],
            tcp_socket_storage: [TcpSocketStorage::new(); NUM_TCP_SOCKETS],
            udp_socket_storage: [UdpSocketStorage::new(); NUM_UDP_SOCKETS],
        }
    }
}

pub type NetworkStack = smoltcp_nal::NetworkStack<'static, 'static, &'static mut Eth>;

pub struct NetworkDevices {
    pub stack: NetworkStack,
    pub mac_address: smoltcp::wire::EthernetAddress,
}

pub struct Thermostat {
    pub network_devices: NetworkDevices,
    pub leds: Leds,
    pub adc: Adc,
    pub dacs: Dacs,
    pub pwms: Pwms,
}

pub fn setup(core: rtic::Peripherals, device: stm32_eth::stm32::Peripherals) -> Thermostat {
    // setup Logger
    static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Trace);
    rtt_target::rtt_init_print!();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
    info!("---Starting Setup");

    let mut cp = core;
    cp.SCB.enable_icache();
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    let dp = device;

    dp.DBGMCU.cr.modify(|_, w| {
        w.dbg_sleep().set_bit();
        w.dbg_standby().set_bit();
        w.dbg_stop().set_bit()
    });
    dp.RCC.ahb1enr.modify(|_, w| w.dma1en().enabled());

    let clocks = dp
        .RCC
        .constrain()
        .cfgr
        .use_hse(HSE)
        .sysclk(168.mhz())
        .hclk(168.mhz())
        .pclk1(32.mhz())
        .pclk2(64.mhz())
        .freeze();

    // take gpios
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();
    let gpiod = dp.GPIOD.split();
    let gpioe = dp.GPIOE.split();
    let gpiog = dp.GPIOG.split();
    let gpiof = dp.GPIOF.split();

    let tim1 = dp.TIM1;
    let tim3 = dp.TIM3;

    let mut leds = Leds::new(
        gpiod.pd9,
        gpiod.pd10.into_push_pull_output(),
        gpiod.pd11.into_push_pull_output(),
    );

    leds.r1.on();
    leds.g3.on();
    leds.g4.off();

    // Setup ethernet.
    info!("Setup ethernet");

    let eth_pins = EthPins {
        ref_clk: gpioa.pa1,
        md_io: gpioa.pa2,
        md_clk: gpioc.pc1,
        crs: gpioa.pa7,
        tx_en: gpiob.pb11,
        tx_d0: gpiog.pg13,
        tx_d1: gpiob.pb13,
        rx_d0: gpioc.pc4,
        rx_d1: gpioc.pc5,
    };

    let eth = {
        static mut RX_RING: Option<[RingEntry<RxDescriptor>; 4]> = None;
        static mut TX_RING: Option<[RingEntry<TxDescriptor>; 4]> = None;
        static mut ETH: Option<Eth> = None;
        unsafe {
            RX_RING = Some(Default::default());
            TX_RING = Some(Default::default());
            info!("Creating ethernet");
            let eth = Eth::new(
                dp.ETHERNET_MAC,
                dp.ETHERNET_DMA,
                &mut RX_RING.as_mut().unwrap()[..],
                &mut TX_RING.as_mut().unwrap()[..],
                PhyAddress::_0,
                clocks,
                eth_pins,
            )
            .unwrap();
            info!("Created ethernet");
            ETH = Some(eth);
            ETH.as_mut().unwrap()
        }
    };

    info!("Enabling ethernet interrupt");
    eth.enable_interrupt();

    let store = cortex_m::singleton!(: NetStorage = NetStorage::default()).unwrap();

    let neighbor_cache = smoltcp::iface::NeighborCache::new(&mut store.neighbor_cache[..]);

    let mut routes = Routes::new(&mut store.routes_cache[..]);
    routes
        .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
        .unwrap();

    info!("Setup interface");

    let ethernet_addr = EthernetAddress(SRC_MAC);
    let interface = InterfaceBuilder::new(eth)
        .ethernet_addr(ethernet_addr)
        .ip_addrs(&mut store.ip_addrs[..])
        .neighbor_cache(neighbor_cache)
        .routes(routes)
        .finalize();

    info!("Setup sockets");
    let sockets = {
        let mut sockets = smoltcp::socket::SocketSet::new(&mut store.sockets[..]);

        for storage in store.tcp_socket_storage[..].iter_mut() {
            let tcp_socket = {
                let rx_buffer = smoltcp::socket::TcpSocketBuffer::new(&mut storage.rx_storage[..]);
                let tx_buffer = smoltcp::socket::TcpSocketBuffer::new(&mut storage.tx_storage[..]);

                smoltcp::socket::TcpSocket::new(rx_buffer, tx_buffer)
            };
            sockets.add(tcp_socket);
        }
        for storage in store.udp_socket_storage[..].iter_mut() {
            let udp_socket = {
                let rx_buffer = smoltcp::socket::UdpSocketBuffer::new(
                    &mut storage.rx_metadata[..],
                    &mut storage.rx_storage[..],
                );
                let tx_buffer = smoltcp::socket::UdpSocketBuffer::new(
                    &mut storage.tx_metadata[..],
                    &mut storage.tx_storage[..],
                );

                smoltcp::socket::UdpSocket::new(rx_buffer, tx_buffer)
            };
            sockets.add(udp_socket);
        }

        sockets
    };

    info!("Setup network stack");
    let stack = smoltcp_nal::NetworkStack::new(interface, sockets);

    let network_devices = NetworkDevices {
        stack,
        mac_address: ethernet_addr,
    };

    info!("Setup ADC");
    let adc_pins = AdcPins {
        sck: gpiob.pb10.into_alternate_af5(),
        miso: gpiob.pb14.into_alternate_af5(),
        mosi: gpiob.pb15.into_alternate_af5(),
        sync: gpiob.pb12.into_push_pull_output(),
    };
    let adc = Adc::new(clocks, dp.SPI2, adc_pins);

    info!("Setup DACs");
    let dac0_pins = Dac0Pins {
        sck: gpioe.pe2.into_alternate_af5(),
        mosi: gpioe.pe6.into_alternate_af5(),
        sync: gpioe.pe4.into_push_pull_output(),
    };

    let dac1_pins = Dac1Pins {
        sck: gpiof.pf7.into_alternate_af5(),
        mosi: gpiof.pf9.into_alternate_af5(),
        sync: gpiof.pf6.into_push_pull_output(),
    };

    let dacs = Dacs::new(clocks, dp.SPI4, dp.SPI5, dac0_pins, dac1_pins);

    let mut pwms = Pwms::new(
        clocks,
        tim1,
        tim3,
        gpioc.pc6,
        gpioc.pc7,
        gpioe.pe9,
        gpioe.pe11,
        gpioe.pe13,
        gpioe.pe14,
        gpioe.pe10.into_push_pull_output(),
        gpioe.pe15.into_push_pull_output(),
    );

    pwms.shdn0.set_high().unwrap();
    pwms.shdn1.set_high().unwrap();

    leds.r1.off();
    info!("---Setup Done");

    let thermostat = Thermostat {
        network_devices,
        leds,
        adc,
        dacs,
        pwms,
    };

    thermostat
}
