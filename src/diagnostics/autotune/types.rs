#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockCheckType {
    DnsSpoof,
    TcpRst,
    SniBlock,
    SiberianBlock,
    QuicBlock,
    CidrWhitelist,
}

impl BlockCheckType {
    pub fn all() -> &'static [Self] {
        &[
            Self::DnsSpoof,
            Self::TcpRst,
            Self::SniBlock,
            Self::SiberianBlock,
            Self::QuicBlock,
            Self::CidrWhitelist,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::DnsSpoof => "DNS",
            Self::TcpRst => "TCP RST",
            Self::SniBlock => "SNI",
            Self::SiberianBlock => "SIBERIAN",
            Self::QuicBlock => "QUIC",
            Self::CidrWhitelist => "CIDR",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockChecks {
    pub enabled: Vec<bool>,
}

impl BlockChecks {
    pub fn all_enabled() -> Self {
        Self { enabled: vec![true; 6] }
    }

    pub fn get(&self, idx: usize) -> bool {
        self.enabled.get(idx).copied().unwrap_or(false)
    }

    pub fn set(&mut self, idx: usize, val: bool) {
        if let Some(e) = self.enabled.get_mut(idx) {
            *e = val;
        }
    }

    pub fn count_enabled(&self) -> usize {
        self.enabled.iter().filter(|&&e| e).count()
    }
}

#[derive(Debug, Clone)]
pub struct AutotuneConfig {
    pub preset_indices: Vec<usize>,
    pub num_requests: usize,
    pub check_http: bool,
    pub check_tls12: bool,
    pub check_tls13: bool,
    pub check_quic: bool,
    pub strategy_indices: Vec<usize>,
    pub block_checks: BlockChecks,
}

impl Default for AutotuneConfig {
    fn default() -> Self {
        Self {
            preset_indices: vec![0],
            num_requests: 3,
            check_http: true,
            check_tls12: true,
            check_tls13: true,
            check_quic: true,
            strategy_indices: Vec::new(),
            block_checks: BlockChecks::all_enabled(),
        }
    }
}
