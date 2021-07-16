///! Thermostat setup
use core::sync::atomic::{self, AtomicBool, Ordering};
use core::{ptr, slice};
use stm32f4xx_hal::{
    self as hal,
    prelude::*,
};

use smoltcp_nal::smoltcp;

use rtt_logger::RTTLogger;

// use stm32_eth::{
//     {EthPins, PhyAddress, RingEntry, RxDescriptor, TxDescriptor},
//     hal::gpio::GpioExt,
//     hal::rcc::RccExt,
//     hal::time::U32Ext as TimeU32Ext,
//     stm32::{Interrupt, CorePeripherals, Peripherals, SYST},
// };


// // use embedded_hal::digital::v2::{InputPin, OutputPin};
//
// // use super::{
// //     adc, afe, cycle_counter::CycleCounter, dac, design_parameters, eeprom,
// //     input_stamper::InputStamper, pounder, pounder::dds_output::DdsOutput,
// //     system_timer, timers, DigitalInput0, DigitalInput1, EthernetPhy,
// //     NetworkStack, AFE0, AFE1,
// // };
//

// const CYCLE_HZ: u32 = 168_000_000;
//
// const NUM_TCP_SOCKETS: usize = 4;
// const NUM_UDP_SOCKETS: usize = 1;
// const NUM_SOCKETS: usize = NUM_UDP_SOCKETS + NUM_TCP_SOCKETS;
//
// type Eth = stm32_eth::Eth<'static, 'static>;
//
// // pub type NetworkStack = smoltcp_nal::NetworkStack<
// //     'static,
// //     'static,
// //     Eth,
// // >;
//
//
// pub struct NetStorage {
//     pub ip_addrs: [smoltcp::wire::IpCidr; 1],
//
//     // Note: There is an additional socket set item required for the DHCP socket.
//     pub sockets:
//         [Option<smoltcp::socket::SocketSetItem<'static>>; NUM_SOCKETS + 1],
//     pub tcp_socket_storage: [TcpSocketStorage; NUM_TCP_SOCKETS],
//     pub udp_socket_storage: [UdpSocketStorage; NUM_UDP_SOCKETS],
//     pub neighbor_cache:
//         [Option<(smoltcp::wire::IpAddress, smoltcp::iface::Neighbor)>; 8],
//     pub routes_cache:
//         [Option<(smoltcp::wire::IpCidr, smoltcp::iface::Route)>; 8],
//
//     pub dhcp_rx_metadata: [smoltcp::socket::RawPacketMetadata; 1],
//     pub dhcp_tx_metadata: [smoltcp::socket::RawPacketMetadata; 1],
//     pub dhcp_tx_storage: [u8; 600],
//     pub dhcp_rx_storage: [u8; 600],
// }
//
// pub struct UdpSocketStorage {
//     rx_storage: [u8; 1024],
//     tx_storage: [u8; 2048],
//     tx_metadata:
//         [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
//     rx_metadata:
//         [smoltcp::storage::PacketMetadata<smoltcp::wire::IpEndpoint>; 10],
// }
//
// impl UdpSocketStorage {
//     const fn new() -> Self {
//         Self {
//             rx_storage: [0; 1024],
//             tx_storage: [0; 2048],
//             tx_metadata: [smoltcp::storage::PacketMetadata::<
//                 smoltcp::wire::IpEndpoint,
//             >::EMPTY; 10],
//             rx_metadata: [smoltcp::storage::PacketMetadata::<
//                 smoltcp::wire::IpEndpoint,
//             >::EMPTY; 10],
//         }
//     }
// }
//
// #[derive(Copy, Clone)]
// pub struct TcpSocketStorage {
//     rx_storage: [u8; 1024],
//     tx_storage: [u8; 1024],
// }
//
// impl TcpSocketStorage {
//     const fn new() -> Self {
//         Self {
//             rx_storage: [0; 1024],
//             tx_storage: [0; 1024],
//         }
//     }
// }
//
// impl NetStorage {
//     pub fn new() -> Self {
//         NetStorage {
//             // Placeholder for the real IP address, which is initialized at runtime.
//             ip_addrs: [smoltcp::wire::IpCidr::Ipv6(
//                 smoltcp::wire::Ipv6Cidr::SOLICITED_NODE_PREFIX,
//             )],
//             neighbor_cache: [None; 8],
//             routes_cache: [None; 8],
//             sockets: [None, None, None, None, None, None],
//             tcp_socket_storage: [TcpSocketStorage::new(); NUM_TCP_SOCKETS],
//             udp_socket_storage: [UdpSocketStorage::new(); NUM_UDP_SOCKETS],
//             dhcp_tx_storage: [0; 600],
//             dhcp_rx_storage: [0; 600],
//             dhcp_rx_metadata: [smoltcp::socket::RawPacketMetadata::EMPTY; 1],
//             dhcp_tx_metadata: [smoltcp::socket::RawPacketMetadata::EMPTY; 1],
//         }
//     }
// }

// /// The available networking devices on Stabilizer.
// pub struct NetworkDevices {
//     pub stack: NetworkStack,
//     pub mac_address: smoltcp::wire::EthernetAddress,
// }


pub fn setup_rtt(){

    static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Info);
    rtt_target::rtt_init_print!();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
    log::info!("Starting");
}

//
// pub fn setup_network(
//     mut core: rtic::Peripherals,
//     device: stm32_eth::stm32::Peripherals,
// ) { //-> NetworkDevices {
//
//     // Setup clocks.
//     let rcc = device.RCC.constrain();
//     let clocks = rcc.cfgr.sysclk(CYCLE_HZ).freeze();
//
//     let gpioa = device.GPIOA.split();
//     let gpiob = device.GPIOB.split();
//     let gpioc = device.GPIOC.split();
//     let eth_pins = EthPins {
//         ref_clk: gpioa.pa1,
//         md_io: gpioa.pa2,
//         md_clk: gpioc.pc1,
//         crs: gpioa.pa7,
//         tx_en: gpiob.pb11,
//         tx_d0: gpiob.pb12,
//         tx_d1: gpiob.pb13,
//         rx_d0: gpioc.pc4,
//         rx_d1: gpioc.pc5,
//     };
//     let eth = {
//         static mut RX_RING: Option<[RingEntry<RxDescriptor>; 8]> = None;
//         static mut TX_RING: Option<[RingEntry<TxDescriptor>; 2]> = None;
//         static mut ETH: Option<Eth> = None;
//         unsafe {
//             RX_RING = Some(Default::default());
//             TX_RING = Some(Default::default());
//             log::info!("Creating ethernet");
//             let (eth) = Eth::new(
//                 device.ETHERNET_MAC,
//                 device.ETHERNET_DMA,
//                 &mut RX_RING.as_mut().unwrap()[..],
//                 &mut TX_RING.as_mut().unwrap()[..],
//                 PhyAddress::_0,
//                 clocks,
//                 eth_pins,
//             ).unwrap();
//             log::info!("Created ethernet");
//             ETH = Some(eth);
//             ETH.as_mut().unwrap()
//         }
//     };
//     log::info!("Enabling interrupt");
//     eth.enable_interrupt();
//
//
//     // let mut stack = smoltcp_nal::NetworkStack::new(interface, sockets);
// }

//     // Configure ethernet pins.
//     {
//         // Reset the PHY before configuring pins.
//         let mut eth_phy_nrst = gpioe.pe3.into_push_pull_output();
//         eth_phy_nrst.set_low().unwrap();
//         delay.delay_us(200u8);
//         eth_phy_nrst.set_high().unwrap();
//
//         let gpioa = gpioa.split();
//         let gpiob = gpiob.split();
//         let gpioc = gpioc.split();
//         let gpioe = gpioe.split();
//         let gpiof = gpiof.split();
//         let gpiog = gpiog.split();
//
//         let eth_pins = EthPins {
//             ref_clk: gpioa.pa1,
//             md_io: gpioa.pa2,
//             md_clk: gpioc.pc1,
//             crs: gpioa.pa7,
//             tx_en: gpiob.pb11,
//             tx_d0: gpiog.pg13,
//             tx_d1: gpiob.pb13,
//             rx_d0: gpioc.pc4,
//             rx_d1: gpioc.pc5,
//         };
//
//     let mac_addr = smoltcp::wire::EthernetAddress(eeprom::read_eui48(
//         &mut eeprom_i2c,
//         &mut delay,
//     ));
//     log::info!("EUI48: {}", mac_addr);
//
//     let network_devices = {
//         // Configure the ethernet controller
//         let (eth_dma, eth_mac) = unsafe {
//             ethernet::new_unchecked(
//                 device.ETHERNET_MAC,
//                 device.ETHERNET_MTL,
//                 device.ETHERNET_DMA,
//                 &mut DES_RING,
//                 mac_addr,
//                 ccdr.peripheral.ETH1MAC,
//                 &ccdr.clocks,
//             )
//         };
//
//         // Reset and initialize the ethernet phy.
//         let mut lan8742a =
//             ethernet::phy::LAN8742A::new(eth_mac.set_phy_addr(0));
//         lan8742a.phy_reset();
//         lan8742a.phy_init();
//
//         unsafe { ethernet::enable_interrupt() };
//
//         // Note(unwrap): The hardware configuration function is only allowed to be called once.
//         // Unwrapping is intended to panic if called again to prevent re-use of global memory.
//         let store =
//             cortex_m::singleton!(: NetStorage = NetStorage::new()).unwrap();
//
//         store.ip_addrs[0] = smoltcp::wire::IpCidr::new(
//             smoltcp::wire::IpAddress::Ipv4(
//                 smoltcp::wire::Ipv4Address::UNSPECIFIED,
//             ),
//             0,
//         );
//
//         let mut routes =
//             smoltcp::iface::Routes::new(&mut store.routes_cache[..]);
//         routes
//             .add_default_ipv4_route(smoltcp::wire::Ipv4Address::UNSPECIFIED)
//             .unwrap();
//
//         let neighbor_cache =
//             smoltcp::iface::NeighborCache::new(&mut store.neighbor_cache[..]);
//
//         let interface = smoltcp::iface::InterfaceBuilder::new(eth_dma)
//             .ethernet_addr(mac_addr)
//             .neighbor_cache(neighbor_cache)
//             .ip_addrs(&mut store.ip_addrs[..])
//             .routes(routes)
//             .finalize();
//
//         let sockets = {
//             let mut sockets =
//                 smoltcp::socket::SocketSet::new(&mut store.sockets[..]);
//
//             for storage in store.tcp_socket_storage[..].iter_mut() {
//                 let tcp_socket = {
//                     let rx_buffer = smoltcp::socket::TcpSocketBuffer::new(
//                         &mut storage.rx_storage[..],
//                     );
//                     let tx_buffer = smoltcp::socket::TcpSocketBuffer::new(
//                         &mut storage.tx_storage[..],
//                     );
//
//                     smoltcp::socket::TcpSocket::new(rx_buffer, tx_buffer)
//                 };
//                 sockets.add(tcp_socket);
//             }
//
//             for storage in store.udp_socket_storage[..].iter_mut() {
//                 let udp_socket = {
//                     let rx_buffer = smoltcp::socket::UdpSocketBuffer::new(
//                         &mut storage.rx_metadata[..],
//                         &mut storage.rx_storage[..],
//                     );
//                     let tx_buffer = smoltcp::socket::UdpSocketBuffer::new(
//                         &mut storage.tx_metadata[..],
//                         &mut storage.tx_storage[..],
//                     );
//
//                     smoltcp::socket::UdpSocket::new(rx_buffer, tx_buffer)
//                 };
//                 sockets.add(udp_socket);
//             }
//
//             sockets.add(smoltcp::socket::Dhcpv4Socket::new());
//
//             sockets
//         };
//
//         let random_seed = {
//             let mut rng =
//                 device.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);
//             let mut data = [0u8; 4];
//             rng.fill(&mut data).unwrap();
//             data
//         };
//
//         let mut stack = smoltcp_nal::NetworkStack::new(interface, sockets);
//
//         stack.seed_random_port(&random_seed);
//
//         NetworkDevices {
//             stack,
//             phy: lan8742a,
//             mac_address: mac_addr,
//         }
//     };
// }
