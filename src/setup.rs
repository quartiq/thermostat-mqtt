
//use panic_halt as _;
use log::{error, info, warn};

use crate::{
    leds::Leds,
};


use smoltcp_nal::smoltcp;
use smoltcp_nal::smoltcp::{
    iface::{InterfaceBuilder, Neighbor, NeighborCache, Routes, Interface},
    socket::{SocketHandle, SocketSetItem, TcpSocket, TcpSocketBuffer},
    // time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address},
};

use stm32_eth::{
    {EthPins, PhyAddress, RingEntry, RxDescriptor, TxDescriptor},
    hal::gpio::GpioExt,
    hal::rcc::RccExt,
    hal::delay::Delay,
    hal::time::{U32Ext, MegaHertz},
    stm32::{Interrupt, CorePeripherals, Peripherals, SYST},
};


use rtt_logger::RTTLogger;

const HSE: MegaHertz = MegaHertz(8);


use rtic::cyccnt::{Instant, U32Ext as _};


type Eth = stm32_eth::Eth<'static, 'static>;

// const SRC_MAC: [u8; 6] = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
// const SRC_MAC: [u8; 6] = [0xF6, 0x48, 0x74, 0xC8, 0xC4, 0x83];
const SRC_MAC: [u8; 6] = [0x80, 0x1f, 0x12, 0x63, 0x84, 0x1a];  // eeprom

const NUM_TCP_SOCKETS: usize = 4;
const NUM_UDP_SOCKETS: usize = 1;
const NUM_SOCKETS: usize = NUM_UDP_SOCKETS + NUM_TCP_SOCKETS;

pub struct NetStorage {
    pub ip_addrs: [smoltcp::wire::IpCidr; 1],

    // Note: There is an additional socket set item required for the DHCP socket.
    pub sockets:
        [Option<smoltcp::socket::SocketSetItem<'static>>; NUM_SOCKETS],
    pub tcp_socket_storage: [TcpSocketStorage; NUM_TCP_SOCKETS],
    pub udp_socket_storage: [UdpSocketStorage; NUM_UDP_SOCKETS],
    pub neighbor_cache:
        [Option<(smoltcp::wire::IpAddress, smoltcp::iface::Neighbor)>; 4],
    pub routes_cache:
        [Option<(smoltcp::wire::IpCidr, smoltcp::iface::Route)>; 4],

}

pub struct UdpSocketStorage {
    rx_storage: [u8; 128],
    tx_storage: [u8; 128],
    tx_metadata:
        [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
    rx_metadata:
        [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
}

impl UdpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; 128],
            tx_storage: [0; 128],
            tx_metadata: [smoltcp::storage::PacketMetadata::<
                smoltcp::wire::IpEndpoint,
            >::EMPTY; 10],
            rx_metadata: [smoltcp::storage::PacketMetadata::<
                smoltcp::wire::IpEndpoint,
            >::EMPTY; 10],
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
            ip_addrs: [IpCidr::new(IpAddress::from(Ipv4Address::new(192, 168, 1, 50)), 24)],
            neighbor_cache: [None; 4],
            routes_cache: [None; 4],
            sockets: [None, None, None, None, None],
            tcp_socket_storage: [TcpSocketStorage::new(); NUM_TCP_SOCKETS],
            udp_socket_storage: [UdpSocketStorage::new(); NUM_UDP_SOCKETS],
        }
    }
}

pub type NetworkStack = smoltcp_nal::NetworkStack<
    'static,
    'static,
    &'static mut Eth,
>;

pub struct NetworkDevices {
    pub stack: NetworkStack,
    pub mac_address: smoltcp::wire::EthernetAddress,
}

pub fn setup(
    mut core: rtic::Peripherals,
    device: stm32_eth::stm32::Peripherals,
) -> (Leds, NetworkDevices) {

    let mut cp = core;
    cp.SCB.enable_icache();
    // cp.SCB.enable_dcache(&mut cp.CPUID);
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    let dp = device;

    dp.DBGMCU.cr.modify(|_, w| {
        w.dbg_sleep().set_bit();
        w.dbg_standby().set_bit();
        w.dbg_stop().set_bit()
    });
    dp.RCC.ahb1enr.modify(|_, w| w.dma1en().enabled());


    let clocks = dp.RCC.constrain()
        .cfgr
        .use_hse(HSE)
        .sysclk(168.mhz())
        .hclk(168.mhz())
        .pclk1(32.mhz())
        .pclk2(64.mhz())
        .freeze();


    // setup Logger
    static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Trace);
    rtt_target::rtt_init_print!();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
    log::trace!("Starting");

    // take gpios
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();
    let gpiod = dp.GPIOD.split();
    let gpiog = dp.GPIOG.split();

    log::trace!("waiting a bit");

    let mut leds = Leds::new(gpiod.pd9, gpiod.pd10.into_push_pull_output(), gpiod.pd11.into_push_pull_output());

    for _ in 0..100000{
        leds.g3.on();
        leds.g3.off();
    }

    leds.r1.on();
    leds.g3.on();
    leds.g4.off();
    log::trace!("waited a bit");



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
            ).unwrap();
            info!("Created ethernet");
            ETH = Some(eth);
            ETH.as_mut().unwrap()
        }
    };

    info!("Enabling ethernet interrupt");
    eth.enable_interrupt();


    let store =
        cortex_m::singleton!(: NetStorage = NetStorage::default()).unwrap();


    let neighbor_cache =
        smoltcp::iface::NeighborCache::new(&mut store.neighbor_cache[..]);

    // let i = match store.ip_addrs[0].address() {
    //     IpAddress::Ipv4(addr) => addr,
    //     _ => unreachable!(),
    // };

    let mut routes = Routes::new(&mut store.routes_cache[..]);
    routes
        // .add_default_ipv4_route(i)
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
        let mut sockets =
            smoltcp::socket::SocketSet::new(&mut store.sockets[..]);

        for storage in store.tcp_socket_storage[..].iter_mut() {
            let tcp_socket = {
                let rx_buffer = smoltcp::socket::TcpSocketBuffer::new(
                    &mut storage.rx_storage[..],
                );
                let tx_buffer = smoltcp::socket::TcpSocketBuffer::new(
                    &mut storage.tx_storage[..],
                );

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
    let mut stack = smoltcp_nal::NetworkStack::new(interface, sockets);

    let mut network_devices = NetworkDevices {
        stack,
        mac_address: ethernet_addr,
    };



    // loop {cortex_m::asm::nop();}

    // let mut leds = Leds::new(gpiod.pd9, gpiod.pd10.into_push_pull_output(), gpiod.pd11.into_push_pull_output());
    //
    // leds.r1.on();
    // leds.g3.on();
    // leds.g4.off();


    (leds, network_devices)

}
