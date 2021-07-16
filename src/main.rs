#![no_std]
#![no_main]

use panic_abort as _;
use log::{error, info, warn};

use smoltcp_nal::smoltcp;
use smoltcp_nal::smoltcp::{
    iface::{InterfaceBuilder, Neighbor, NeighborCache, Routes},
    socket::{SocketHandle, SocketSetItem, TcpSocket, TcpSocketBuffer},
    // time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address},
};

use stm32_eth::{
    {EthPins, PhyAddress, RingEntry, RxDescriptor, TxDescriptor},
    hal::gpio::GpioExt,
    hal::rcc::RccExt,
    hal::time::{U32Ext, MegaHertz},
    stm32::{Interrupt, CorePeripherals, Peripherals, SYST},
};

use crate::{
    leds::Leds,
};

mod leds;

// mod setup;
use rtt_logger::RTTLogger;

const HSE: MegaHertz = MegaHertz(8);

use rtic::cyccnt::{Instant, U32Ext as _};

const PERIOD: u32 = 1<<25;


type Eth = stm32_eth::Eth<'static, 'static>;

const SRC_MAC: [u8; 6] = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];


const NUM_TCP_SOCKETS: usize = 4;
const NUM_UDP_SOCKETS: usize = 1;
const NUM_SOCKETS: usize = NUM_UDP_SOCKETS + NUM_TCP_SOCKETS;

pub struct NetStorage {
    pub ip_addrs: [smoltcp::wire::IpCidr; 1],

    // Note: There is an additional socket set item required for the DHCP socket.
    pub sockets:
        [Option<smoltcp::socket::SocketSetItem<'static>>; NUM_SOCKETS + 1],
    pub tcp_socket_storage: [TcpSocketStorage; NUM_TCP_SOCKETS],
    pub udp_socket_storage: [UdpSocketStorage; NUM_UDP_SOCKETS],
    pub neighbor_cache:
        [Option<(smoltcp::wire::IpAddress, smoltcp::iface::Neighbor)>; 8],
    pub routes_cache:
        [Option<(smoltcp::wire::IpCidr, smoltcp::iface::Route)>; 8],

}

pub struct UdpSocketStorage {
    rx_storage: [u8; 1024],
    tx_storage: [u8; 2048],
    tx_metadata:
        [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
    rx_metadata:
        [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
}

impl UdpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; 1024],
            tx_storage: [0; 2048],
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
    rx_storage: [u8; 1024],
    tx_storage: [u8; 1024],
}

impl TcpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; 1024],
            tx_storage: [0; 1024],
        }
    }
}

impl NetStorage {
    pub fn new() -> Self {
        NetStorage {
            // Placeholder for the real IP address, which is initialized at runtime.
            ip_addrs: [smoltcp::wire::IpCidr::Ipv6(
                smoltcp::wire::Ipv6Cidr::SOLICITED_NODE_PREFIX,
            )],
            neighbor_cache: [None; 8],
            routes_cache: [None; 8],
            sockets: [None, None, None, None, None, None],
            tcp_socket_storage: [TcpSocketStorage::new(); NUM_TCP_SOCKETS],
            udp_socket_storage: [UdpSocketStorage::new(); NUM_UDP_SOCKETS],
        }
    }
}




#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        leds: Leds,
    }

    #[init(schedule = [blink])]
    fn init(c: init::Context) -> init::LateResources {

        let mut cp = c.core;
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);
        cp.DCB.enable_trace();
        cp.DWT.enable_cycle_counter();

        let dp = c.device;

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
        static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Info);
        rtt_target::rtt_init_print!();
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Trace))
            .unwrap();
        log::info!("Starting");

        // take gpios
        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();
        let gpiod = dp.GPIOD.split();

        // Setup ethernet.
        info!("Setup ethernet");

        let eth_pins = EthPins {
            ref_clk: gpioa.pa1,
            md_io: gpioa.pa2,
            md_clk: gpioc.pc1,
            crs: gpioa.pa7,
            tx_en: gpiob.pb11,
            tx_d0: gpiob.pb12,
            tx_d1: gpiob.pb13,
            rx_d0: gpioc.pc4,
            rx_d1: gpioc.pc5,
        };

        let eth = {
            static mut RX_RING: Option<[RingEntry<RxDescriptor>; 8]> = None;
            static mut TX_RING: Option<[RingEntry<TxDescriptor>; 2]> = None;
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


        // Setup TCP/IP.
        info!("Setup TCP/IP");
        let local_addr = Ipv4Address::new(10, 0, 0, 1);
        let ip_addr = IpCidr::new(IpAddress::from(local_addr), 24);
        let ip_addrs = {
            static mut IP_ADDRS: Option<[IpCidr; 1]> = None;
            unsafe {
                IP_ADDRS = Some([ip_addr]);
                IP_ADDRS.as_mut().unwrap()
            }
        };

        let store =
            cortex_m::singleton!(: NetStorage = NetStorage::new()).unwrap();


        let neighbor_cache =
            smoltcp::iface::NeighborCache::new(&mut store.neighbor_cache[..]);


        let mut routes = Routes::new(&mut store.routes_cache[..]);
        routes
            .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
            .unwrap();

        let ethernet_addr = EthernetAddress(SRC_MAC);
        let interface = InterfaceBuilder::new(eth)
            .ethernet_addr(ethernet_addr)
            .ip_addrs(&mut ip_addrs[..])
            .neighbor_cache(neighbor_cache)
            .routes(routes)
            .finalize();


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

            sockets.add(smoltcp::socket::Dhcpv4Socket::new());

            sockets
        };

        let mut stack = smoltcp_nal::NetworkStack::new(interface, sockets);

        //
        // NetworkDevices {
        //     stack,
        //     phy: lan8742a,
        //     mac_address: mac_addr,
        // }

        let mut leds = Leds::new(gpiod.pd9, gpiod.pd10.into_push_pull_output(), gpiod.pd11.into_push_pull_output());

        leds.r1.on();
        leds.g3.on();
        leds.g4.off();

        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();


        init::LateResources {
            leds: leds
        }
    }

    #[task(resources = [leds], schedule = [blink])]
    fn blink(c: blink::Context) {
        static mut LED_STATE: bool = false;

        if *LED_STATE {
            c.resources.leds.g3.off();
            *LED_STATE = false;
            log::info!("led off");
        } else {
            c.resources.leds.g3.on();
            *LED_STATE = true;
            log::info!("led on");
        }
        c.schedule.blink(c.scheduled + PERIOD.cycles()).unwrap();

    }

    extern "C" {
        fn EXTI0();
    }
};
