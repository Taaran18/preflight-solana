pub const VERIFY: u8 = 0;
pub const HEADER_LEN: usize = 34;
pub const CHECK_LEN: usize = 10;
pub const MAX_CHECKS: usize = 32;
pub const TOKEN_ACCOUNT_LEN: usize = 165;
pub const ERROR_BASE: u32 = 7000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    TooShort,
    UnknownInstruction,
    BadCheckCount,
    BadLength,
    UnknownCheckKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    TokenMin,
    TokenMax,
    LamportsMin,
    LamportsMax,
    TokenOwner,
    NoDelegate,
}

impl CheckKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::TokenMin),
            1 => Some(Self::TokenMax),
            2 => Some(Self::LamportsMin),
            3 => Some(Self::LamportsMax),
            4 => Some(Self::TokenOwner),
            5 => Some(Self::NoDelegate),
            _ => None,
        }
    }

    pub fn needs_token_account(self) -> bool {
        !matches!(self, Self::LamportsMin | Self::LamportsMax)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub kind: CheckKind,
    pub account: u8,
    pub value: u64,
}

impl Check {
    pub fn decode(chunk: &[u8]) -> Option<Self> {
        if chunk.len() != CHECK_LEN {
            return None;
        }
        let kind = CheckKind::from_u8(chunk[0])?;
        let mut value = [0u8; 8];
        value.copy_from_slice(&chunk[2..10]);
        Some(Self {
            kind,
            account: chunk[1],
            value: u64::from_le_bytes(value),
        })
    }
}

pub struct Parsed<'a> {
    pub receipt: &'a [u8],
    body: &'a [u8],
}

impl<'a> Parsed<'a> {
    pub fn len(&self) -> usize {
        self.body.len() / CHECK_LEN
    }

    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    pub fn checks(&self) -> impl Iterator<Item = Check> + 'a {
        self.body.chunks_exact(CHECK_LEN).filter_map(Check::decode)
    }
}

pub fn parse(data: &[u8]) -> Result<Parsed<'_>, GuardError> {
    if data.len() < HEADER_LEN {
        return Err(GuardError::TooShort);
    }
    if data[0] != VERIFY {
        return Err(GuardError::UnknownInstruction);
    }
    let count = data[33] as usize;
    if count == 0 || count > MAX_CHECKS {
        return Err(GuardError::BadCheckCount);
    }
    let body = &data[HEADER_LEN..];
    if body.len() != count * CHECK_LEN {
        return Err(GuardError::BadLength);
    }
    for chunk in body.chunks_exact(CHECK_LEN) {
        if CheckKind::from_u8(chunk[0]).is_none() {
            return Err(GuardError::UnknownCheckKind);
        }
    }
    Ok(Parsed {
        receipt: &data[1..33],
        body,
    })
}

pub struct TokenView<'a>(&'a [u8]);

impl<'a> TokenView<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < TOKEN_ACCOUNT_LEN {
            return None;
        }
        if data.len() > TOKEN_ACCOUNT_LEN && data[TOKEN_ACCOUNT_LEN] != 2 {
            return None;
        }
        Some(Self(data))
    }

    pub fn mint(&self) -> &[u8] {
        &self.0[0..32]
    }

    pub fn owner(&self) -> &[u8] {
        &self.0[32..64]
    }

    pub fn amount(&self) -> u64 {
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&self.0[64..72]);
        u64::from_le_bytes(raw)
    }

    pub fn has_delegate(&self) -> bool {
        self.0[72..76] != [0, 0, 0, 0]
    }
}

pub fn token_check(kind: CheckKind, view: &TokenView, value: u64, expected_owner: Option<&[u8]>) -> bool {
    match kind {
        CheckKind::TokenMin => view.amount() >= value,
        CheckKind::TokenMax => view.amount() <= value,
        CheckKind::TokenOwner => expected_owner.is_some_and(|owner| view.owner() == owner),
        CheckKind::NoDelegate => !view.has_delegate(),
        CheckKind::LamportsMin | CheckKind::LamportsMax => false,
    }
}

pub fn lamports_check(kind: CheckKind, lamports: u64, value: u64) -> bool {
    match kind {
        CheckKind::LamportsMin => lamports >= value,
        CheckKind::LamportsMax => lamports <= value,
        _ => false,
    }
}

pub fn hex32(bytes: &[u8], out: &mut [u8; 64]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, byte) in bytes.iter().take(32).enumerate() {
        out[i * 2] = HEX[(byte >> 4) as usize];
        out[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
}

pub fn write_u32(value: u32, out: &mut [u8; 10]) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut digits = [0u8; 10];
    let mut n = value;
    let mut len = 0;
    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for i in 0..len {
        out[i] = digits[len - 1 - i];
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn encode(receipt: [u8; 32], checks: &[(u8, u8, u64)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(VERIFY);
        data.extend_from_slice(&receipt);
        data.push(checks.len() as u8);
        for (kind, account, value) in checks {
            data.push(*kind);
            data.push(*account);
            data.extend_from_slice(&value.to_le_bytes());
        }
        data
    }

    fn token_account(owner: [u8; 32], amount: u64, delegate: bool, extended: bool) -> Vec<u8> {
        let mut data = std::vec![0u8; if extended { 170 } else { 165 }];
        data[32..64].copy_from_slice(&owner);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        if delegate {
            data[72] = 1;
        }
        if extended {
            data[165] = 2;
        }
        data
    }

    #[test]
    fn parses_valid_instruction() {
        let data = encode([7u8; 32], &[(0, 1, 500), (2, 0, 1_000_000)]);
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.receipt, &[7u8; 32]);
        let checks: Vec<Check> = parsed.checks().collect();
        assert_eq!(checks[0], Check { kind: CheckKind::TokenMin, account: 1, value: 500 });
        assert_eq!(checks[1], Check { kind: CheckKind::LamportsMin, account: 0, value: 1_000_000 });
    }

    #[test]
    fn rejects_malformed_instructions() {
        assert_eq!(parse(&[0u8; 10]).err(), Some(GuardError::TooShort));
        let mut data = encode([0u8; 32], &[(0, 1, 1)]);
        data[0] = 9;
        assert_eq!(parse(&data).err(), Some(GuardError::UnknownInstruction));
        let data = encode([0u8; 32], &[]);
        assert_eq!(parse(&data).err(), Some(GuardError::BadCheckCount));
        let mut data = encode([0u8; 32], &[(0, 1, 1)]);
        data.push(0);
        assert_eq!(parse(&data).err(), Some(GuardError::BadLength));
        let data = encode([0u8; 32], &[(42, 1, 1)]);
        assert_eq!(parse(&data).err(), Some(GuardError::UnknownCheckKind));
    }

    #[test]
    fn evaluates_token_checks() {
        let owner = [3u8; 32];
        let raw = token_account(owner, 1_000, false, false);
        let view = TokenView::new(&raw).unwrap();
        assert!(token_check(CheckKind::TokenMin, &view, 1_000, None));
        assert!(!token_check(CheckKind::TokenMin, &view, 1_001, None));
        assert!(token_check(CheckKind::TokenMax, &view, 1_000, None));
        assert!(!token_check(CheckKind::TokenMax, &view, 999, None));
        assert!(token_check(CheckKind::TokenOwner, &view, 0, Some(&owner)));
        assert!(!token_check(CheckKind::TokenOwner, &view, 0, Some(&[4u8; 32])));
        assert!(token_check(CheckKind::NoDelegate, &view, 0, None));
        let delegated = token_account(owner, 1_000, true, true);
        let view = TokenView::new(&delegated).unwrap();
        assert!(!token_check(CheckKind::NoDelegate, &view, 0, None));
    }

    #[test]
    fn rejects_non_token_accounts() {
        assert!(TokenView::new(&[0u8; 82]).is_none());
        let mut mint_2022 = std::vec![0u8; 200];
        mint_2022[165] = 1;
        assert!(TokenView::new(&mint_2022).is_none());
    }

    #[test]
    fn evaluates_lamport_checks() {
        assert!(lamports_check(CheckKind::LamportsMin, 10, 10));
        assert!(!lamports_check(CheckKind::LamportsMin, 9, 10));
        assert!(lamports_check(CheckKind::LamportsMax, 10, 10));
        assert!(!lamports_check(CheckKind::LamportsMax, 11, 10));
    }

    #[test]
    fn formats_logs() {
        let mut hex = [0u8; 64];
        let mut bytes = [0u8; 32];
        bytes[0] = 0xab;
        bytes[31] = 0x01;
        hex32(&bytes, &mut hex);
        assert_eq!(&hex[0..2], b"ab");
        assert_eq!(&hex[62..64], b"01");
        let mut digits = [0u8; 10];
        let len = write_u32(7003, &mut digits);
        assert_eq!(&digits[..len], b"7003");
    }
}
