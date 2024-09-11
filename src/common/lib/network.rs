use std::net::IpAddr;

// Function to get the machine's IP address
pub fn get_machine_ip() -> Option<IpAddr> {
    let ifaces = pnet::datalink::interfaces();
    for iface in ifaces {
        for ip in iface.ips {
            if let IpAddr::V4(ipv4) = ip.ip() {
                return Some(IpAddr::V4(ipv4)); // Return the first IPv4 address found
            }
        }
    }
    None
}
