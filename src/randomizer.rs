use std::collections::HashSet;
use std::net::Ipv4Addr;
use futures::stream::{self, Stream, StreamExt};
use rand::Rng;

#[derive(Default, Clone, Copy)]
pub enum IpType {
    #[default]
    Any,
    PublicOnly,
    PrivateOnly,
}

pub struct IpGenerator {
    network_u32: u32,
    range_size: u32,
    count: u32,
    ip_type: IpType,
    use_cidr: bool,
}

impl IpGenerator {
    pub fn generate(self) -> impl Stream<Item = Ipv4Addr> {
        let ip_type = self.ip_type;
        let use_cidr = self.use_cidr;
        let network = self.network_u32;
        let size = self.range_size;

        let max_possible = if use_cidr { size } else { u32::MAX };
        let count = std::cmp::min(self.count, max_possible);

        stream::repeat(())
            .scan(HashSet::with_capacity(count as usize), move |generated, _| {
                let mut rng = rand::rng();
                let mut attempts = 0;
                let max_attempts = 1000;

                loop {
                    let ip = if use_cidr {
                        Ipv4Addr::from(network + (rng.random::<u32>() % size))
                    } else {
                        Ipv4Addr::from(rng.random::<u32>())
                    };

                    if self.is_valid(&ip, ip_type) {
                        if generated.insert(ip) {
                            return std::future::ready(Some(ip));
                        }
                    }

                    attempts += 1;
                    if attempts > max_attempts {
                        return std::future::ready(None);
                    }
                }
            })
            .take(count as usize)
    }

    fn is_valid(&self, ip: &Ipv4Addr, t: IpType) -> bool {
        match t {
            IpType::Any => true,
            IpType::PublicOnly => {
                !ip.is_private() &&
                    !ip.is_loopback() &&
                    !ip.is_unspecified() &&
                    !ip.is_multicast() &&
                    !ip.is_link_local() &&
                    !ip.is_documentation()
            },
            IpType::PrivateOnly => ip.is_private(),
        }
    }

    pub fn builder() -> IpGeneratorBuilder {
        IpGeneratorBuilder::default()
    }
}

#[derive(Default)]
pub struct IpGeneratorBuilder {
    network: Option<Ipv4Addr>,
    prefix_len: Option<u8>,
    count: u32,
    ip_type: IpType,
}

impl IpGeneratorBuilder {
    pub fn cidr(mut self, ip: &str, prefix: u8) -> Self {
        self.network = Some(ip.parse().expect("Invalid IP-Address"));
        self.prefix_len = Some(prefix);
        self
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    pub fn ip_type(mut self, t: IpType) -> Self {
        self.ip_type = t;
        self
    }

    pub fn build(self) -> IpGenerator {
        let use_cidr = self.network.is_some();
        let ip = self.network.unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        let prefix = self.prefix_len.unwrap_or(0);

        let mask = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
        let network_u32 = u32::from(ip) & mask;
        let range_size = if prefix == 0 { u32::MAX } else { 1u32 << (32 - prefix) };

        IpGenerator {
            network_u32,
            range_size,
            count: self.count,
            ip_type: self.ip_type,
            use_cidr,
        }
    }
}