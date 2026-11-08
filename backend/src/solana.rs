use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::LazyLock,
};

use anyhow::{Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use curve25519_dalek::edwards::CompressedEdwardsY;
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};

pub const PACKET_LIMIT: usize = 1232;
pub const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const MAX_COMPUTE_UNITS: u32 = 1_400_000;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.len() < 32 || value.len() > 44 {
            return None;
        }
        let bytes = bs58::decode(value).into_vec().ok()?;
        Self::from_slice(&bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        let array: [u8; 32] = bytes.try_into().ok()?;
        Some(Self(array))
    }

    pub fn is_on_curve(&self) -> bool {
        CompressedEdwardsY(self.0).decompress().is_some()
    }

    pub fn find_program_address(seeds: &[&[u8]], program: &Pubkey) -> Option<(Pubkey, u8)> {
        for bump in (0..=u8::MAX).rev() {
            let mut hasher = Sha256::new();
            for seed in seeds {
                hasher.update(seed);
            }
            hasher.update([bump]);
            hasher.update(program.0);
            hasher.update(b"ProgramDerivedAddress");
            let digest = hasher.finalize();
            let candidate = Pubkey::from_slice(digest.as_slice())?;
            if !candidate.is_on_curve() {
                return Some((candidate, bump));
            }
        }
        None
    }

    pub fn short(&self) -> String {
        let full = self.to_string();
        if full.len() <= 10 {
            return full;
        }
        format!("{}…{}", &full[..4], &full[full.len() - 4..])
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&bs58::encode(self.0).into_string())
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for Pubkey {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

fn key(value: &str) -> Pubkey {
    Pubkey::parse(value).unwrap_or_default()
}

pub static SYSTEM_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| Pubkey([0u8; 32]));
pub static TOKEN_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));
pub static TOKEN_2022_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"));
pub static ATA_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"));
pub static COMPUTE_BUDGET_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("ComputeBudget111111111111111111111111111111"));
pub static MEMO_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"));
pub static LOOKUP_TABLE_PROGRAM: LazyLock<Pubkey> = LazyLock::new(|| key("AddressLookupTab1e1111111111111111111111111"));

pub fn is_token_program(program: &Pubkey) -> bool {
    *program == *TOKEN_PROGRAM || *program == *TOKEN_2022_PROGRAM
}

pub fn program_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "11111111111111111111111111111111" => "System Program",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "SPL Token",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => "Token-2022",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "Associated Token Account",
        "ComputeBudget111111111111111111111111111111" => "Compute Budget",
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr" => "Memo",
        "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo" => "Memo (v1)",
        "AddressLookupTab1e1111111111111111111111111" => "Address Lookup Table",
        "Stake11111111111111111111111111111111111111" => "Stake Program",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter Aggregator",
        "j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X" => "Jupiter Limit Order",
        "DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M" => "Jupiter DCA",
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpools",
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM",
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Raydium CLMM",
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => "Raydium CPMM",
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM",
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => "Meteora Pools",
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => "Meteora DAMM v2",
        "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => "Phoenix",
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => "PumpSwap",
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "Pump.fun",
        "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD" => "Marinade",
        "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy" => "Stake Pool",
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" => "Metaplex Metadata",
        _ => return None,
    })
}

pub fn to_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn from_base64(value: &str) -> Result<Vec<u8>> {
    Ok(STANDARD.decode(value.trim())?)
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_slice());
    out
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn pseudo_random_key(salt: &str) -> Pubkey {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Pubkey(sha256(format!("{salt}:{nanos}").as_bytes()))
}

#[derive(Clone, Debug)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn writable(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    pub fn readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

pub fn set_compute_unit_limit(units: u32) -> Instruction {
    let mut data = vec![2u8];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction {
        program_id: *COMPUTE_BUDGET_PROGRAM,
        accounts: vec![],
        data,
    }
}

pub fn set_compute_unit_price(micro_lamports: u64) -> Instruction {
    let mut data = vec![3u8];
    data.extend_from_slice(&micro_lamports.to_le_bytes());
    Instruction {
        program_id: *COMPUTE_BUDGET_PROGRAM,
        accounts: vec![],
        data,
    }
}

pub fn compute_unit_price(instructions: &[Instruction]) -> Option<u64> {
    instructions
        .iter()
        .find(|ix| ix.program_id == *COMPUTE_BUDGET_PROGRAM && ix.data.first() == Some(&3) && ix.data.len() >= 9)
        .and_then(|ix| ix.data[1..9].try_into().ok().map(u64::from_le_bytes))
}

pub fn with_compute_limit(instructions: &[Instruction], units: u32) -> Vec<Instruction> {
    let mut out = vec![set_compute_unit_limit(units)];
    out.extend(
        instructions
            .iter()
            .filter(|ix| !(ix.program_id == *COMPUTE_BUDGET_PROGRAM && ix.data.first() == Some(&2)))
            .cloned(),
    );
    out
}

pub fn system_transfer(from: Pubkey, to: Pubkey, lamports: u64) -> Instruction {
    let mut data = 2u32.to_le_bytes().to_vec();
    data.extend_from_slice(&lamports.to_le_bytes());
    Instruction {
        program_id: *SYSTEM_PROGRAM,
        accounts: vec![AccountMeta::writable(from, true), AccountMeta::writable(to, false)],
        data,
    }
}

pub fn token_transfer_checked(
    source: Pubkey,
    mint: Pubkey,
    destination: Pubkey,
    owner: Pubkey,
    amount: u64,
    decimals: u8,
    program: Pubkey,
) -> Instruction {
    let mut data = vec![12u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::writable(source, false),
            AccountMeta::readonly(mint, false),
            AccountMeta::writable(destination, false),
            AccountMeta::readonly(owner, true),
        ],
        data,
    }
}

pub fn token_approve(source: Pubkey, delegate: Pubkey, owner: Pubkey, amount: u64, program: Pubkey) -> Instruction {
    let mut data = vec![4u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::writable(source, false),
            AccountMeta::readonly(delegate, false),
            AccountMeta::readonly(owner, true),
        ],
        data,
    }
}

pub fn create_ata_idempotent(payer: Pubkey, ata: Pubkey, owner: Pubkey, mint: Pubkey, token_program: Pubkey) -> Instruction {
    Instruction {
        program_id: *ATA_PROGRAM,
        accounts: vec![
            AccountMeta::writable(payer, true),
            AccountMeta::writable(ata, false),
            AccountMeta::readonly(owner, false),
            AccountMeta::readonly(mint, false),
            AccountMeta::readonly(*SYSTEM_PROGRAM, false),
            AccountMeta::readonly(token_program, false),
        ],
        data: vec![1u8],
    }
}

pub fn memo(text: &str, signer: Pubkey) -> Instruction {
    Instruction {
        program_id: *MEMO_PROGRAM,
        accounts: vec![AccountMeta::readonly(signer, true)],
        data: text.as_bytes().to_vec(),
    }
}

pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Option<Pubkey> {
    Pubkey::find_program_address(&[&owner.0, &token_program.0, &mint.0], &ATA_PROGRAM).map(|(address, _)| address)
}

#[derive(Clone, Debug)]
pub struct LookupTable {
    pub key: Pubkey,
    pub addresses: Vec<Pubkey>,
}

pub fn parse_lookup_table(key: Pubkey, owner: &Pubkey, data: &[u8]) -> Option<LookupTable> {
    if *owner != *LOOKUP_TABLE_PROGRAM || data.len() < 56 || data[0..4] != [1, 0, 0, 0] {
        return None;
    }
    let addresses = data[56..].chunks_exact(32).filter_map(Pubkey::from_slice).collect();
    Some(LookupTable { key, addresses })
}

#[derive(Clone, Debug, Default)]
pub struct Header {
    pub required_signatures: u8,
    pub readonly_signed: u8,
    pub readonly_unsigned: u8,
}

#[derive(Clone, Debug)]
pub struct CompiledInstruction {
    pub program_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Lookup {
    pub table: Pubkey,
    pub writable: Vec<u8>,
    pub readonly: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub versioned: bool,
    pub header: Header,
    pub keys: Vec<Pubkey>,
    pub blockhash: [u8; 32],
    pub instructions: Vec<CompiledInstruction>,
    pub lookups: Vec<Lookup>,
}

#[derive(Clone, Copy, Default)]
struct KeyFlags {
    signer: bool,
    writable: bool,
    invoked: bool,
}

fn touch(order: &mut Vec<Pubkey>, flags: &mut HashMap<Pubkey, KeyFlags>, key: Pubkey, signer: bool, writable: bool, invoked: bool) {
    let entry = flags.entry(key).or_insert_with(|| {
        order.push(key);
        KeyFlags::default()
    });
    entry.signer |= signer;
    entry.writable |= writable;
    entry.invoked |= invoked;
}

impl Message {
    pub fn compile(payer: Pubkey, instructions: &[Instruction], tables: &[LookupTable], blockhash: [u8; 32]) -> Result<Self> {
        let mut order = Vec::new();
        let mut flags: HashMap<Pubkey, KeyFlags> = HashMap::new();
        touch(&mut order, &mut flags, payer, true, true, false);
        for ix in instructions {
            touch(&mut order, &mut flags, ix.program_id, false, false, true);
            for account in &ix.accounts {
                touch(&mut order, &mut flags, account.pubkey, account.is_signer, account.is_writable, false);
            }
        }

        let mut looked_up: HashSet<Pubkey> = HashSet::new();
        let mut lookups = Vec::new();
        let mut loaded_writable = Vec::new();
        let mut loaded_readonly = Vec::new();
        for table in tables {
            let positions: HashMap<Pubkey, usize> = table
                .addresses
                .iter()
                .enumerate()
                .rev()
                .map(|(index, address)| (*address, index))
                .collect();
            let mut writable = Vec::new();
            let mut readonly = Vec::new();
            for key in &order {
                if looked_up.contains(key) {
                    continue;
                }
                let flag = flags[key];
                if flag.signer || flag.invoked {
                    continue;
                }
                let Some(position) = positions.get(key).copied() else {
                    continue;
                };
                if position > u8::MAX as usize {
                    continue;
                }
                if flag.writable {
                    writable.push(position as u8);
                    loaded_writable.push(*key);
                } else {
                    readonly.push(position as u8);
                    loaded_readonly.push(*key);
                }
                looked_up.insert(*key);
            }
            if !writable.is_empty() || !readonly.is_empty() {
                lookups.push(Lookup {
                    table: table.key,
                    writable,
                    readonly,
                });
            }
        }

        let statics: Vec<Pubkey> = order.iter().copied().filter(|key| !looked_up.contains(key)).collect();
        let group = |signer: bool, writable: bool| -> Vec<Pubkey> {
            statics
                .iter()
                .copied()
                .filter(|key| {
                    let flag = flags[key];
                    flag.signer == signer && flag.writable == writable
                })
                .collect()
        };
        let signer_writable = group(true, true);
        let signer_readonly = group(true, false);
        let unsigned_writable = group(false, true);
        let unsigned_readonly = group(false, false);

        let header = Header {
            required_signatures: (signer_writable.len() + signer_readonly.len()) as u8,
            readonly_signed: signer_readonly.len() as u8,
            readonly_unsigned: unsigned_readonly.len() as u8,
        };
        let mut keys = signer_writable;
        keys.extend(signer_readonly);
        keys.extend(unsigned_writable);
        keys.extend(unsigned_readonly);

        let mut all = keys.clone();
        all.extend(loaded_writable);
        all.extend(loaded_readonly);
        if all.len() > 256 {
            bail!("transaction references too many accounts ({})", all.len());
        }
        let index: HashMap<Pubkey, u8> = all.iter().enumerate().map(|(i, key)| (*key, i as u8)).collect();
        let compiled = instructions
            .iter()
            .map(|ix| CompiledInstruction {
                program_index: index[&ix.program_id],
                accounts: ix.accounts.iter().map(|account| index[&account.pubkey]).collect(),
                data: ix.data.clone(),
            })
            .collect();

        Ok(Self {
            versioned: true,
            header,
            keys,
            blockhash,
            instructions: compiled,
            lookups,
        })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1024);
        if self.versioned {
            out.push(0x80);
        }
        out.push(self.header.required_signatures);
        out.push(self.header.readonly_signed);
        out.push(self.header.readonly_unsigned);
        write_compact(&mut out, self.keys.len());
        for key in &self.keys {
            out.extend_from_slice(&key.0);
        }
        out.extend_from_slice(&self.blockhash);
        write_compact(&mut out, self.instructions.len());
        for ix in &self.instructions {
            out.push(ix.program_index);
            write_compact(&mut out, ix.accounts.len());
            out.extend_from_slice(&ix.accounts);
            write_compact(&mut out, ix.data.len());
            out.extend_from_slice(&ix.data);
        }
        if self.versioned {
            write_compact(&mut out, self.lookups.len());
            for lookup in &self.lookups {
                out.extend_from_slice(&lookup.table.0);
                write_compact(&mut out, lookup.writable.len());
                out.extend_from_slice(&lookup.writable);
                write_compact(&mut out, lookup.readonly.len());
                out.extend_from_slice(&lookup.readonly);
            }
        }
        out
    }

    pub fn encode_unsigned(&self) -> Vec<u8> {
        let signatures = self.header.required_signatures as usize;
        let mut out = Vec::with_capacity(1232);
        write_compact(&mut out, signatures);
        out.extend(std::iter::repeat_n(0u8, 64 * signatures));
        out.extend(self.serialize());
        out
    }

    pub fn account_keys(&self, tables: &[LookupTable]) -> Result<Vec<Pubkey>> {
        let mut all = self.keys.clone();
        let mut readonly = Vec::new();
        for lookup in &self.lookups {
            let table = tables
                .iter()
                .find(|table| table.key == lookup.table)
                .ok_or_else(|| anyhow!("missing address lookup table {}", lookup.table))?;
            for index in &lookup.writable {
                all.push(*table.addresses.get(*index as usize).ok_or_else(|| anyhow!("lookup index out of range"))?);
            }
            for index in &lookup.readonly {
                readonly.push(*table.addresses.get(*index as usize).ok_or_else(|| anyhow!("lookup index out of range"))?);
            }
        }
        all.extend(readonly);
        Ok(all)
    }

    pub fn is_signer(&self, index: usize) -> bool {
        index < self.header.required_signatures as usize
    }

    pub fn is_writable(&self, index: usize) -> bool {
        let statics = self.keys.len();
        let required = self.header.required_signatures as usize;
        if index < statics {
            if index < required {
                index < required.saturating_sub(self.header.readonly_signed as usize)
            } else {
                index < statics.saturating_sub(self.header.readonly_unsigned as usize)
            }
        } else {
            let loaded_writable: usize = self.lookups.iter().map(|lookup| lookup.writable.len()).sum();
            index - statics < loaded_writable
        }
    }

    pub fn decompile(&self, tables: &[LookupTable]) -> Result<Vec<Instruction>> {
        let keys = self.account_keys(tables)?;
        self.instructions
            .iter()
            .map(|ix| {
                let program_id = *keys.get(ix.program_index as usize).ok_or_else(|| anyhow!("bad program index"))?;
                let accounts = ix
                    .accounts
                    .iter()
                    .map(|index| {
                        let index = *index as usize;
                        keys.get(index)
                            .map(|pubkey| AccountMeta {
                                pubkey: *pubkey,
                                is_signer: self.is_signer(index),
                                is_writable: self.is_writable(index),
                            })
                            .ok_or_else(|| anyhow!("bad account index"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Instruction {
                    program_id,
                    accounts,
                    data: ix.data.clone(),
                })
            })
            .collect()
    }

    pub fn fee_payer(&self) -> Option<Pubkey> {
        self.keys.first().copied()
    }
}

fn write_compact(out: &mut Vec<u8>, mut value: usize) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        byte |= 0x80;
        out.push(byte);
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8> {
        let value = *self.data.get(self.pos).ok_or_else(|| anyhow!("transaction is truncated"))?;
        self.pos += 1;
        Ok(value)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or_else(|| anyhow!("transaction is malformed"))?;
        let slice = self.data.get(self.pos..end).ok_or_else(|| anyhow!("transaction is truncated"))?;
        self.pos = end;
        Ok(slice)
    }

    fn compact(&mut self) -> Result<usize> {
        let mut value = 0usize;
        for shift in [0, 7, 14] {
            let byte = self.byte()?;
            value |= ((byte & 0x7f) as usize) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        bail!("invalid compact length")
    }

    fn pubkey(&mut self) -> Result<Pubkey> {
        Pubkey::from_slice(self.take(32)?).ok_or_else(|| anyhow!("invalid public key"))
    }

    fn bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.compact()?;
        Ok(self.take(len)?.to_vec())
    }
}

pub fn decode_transaction(bytes: &[u8]) -> Result<Message> {
    let mut reader = Reader { data: bytes, pos: 0 };
    let signatures = reader.compact()?;
    reader.take(signatures * 64)?;
    let first = *bytes.get(reader.pos).ok_or_else(|| anyhow!("transaction has no message"))?;
    let versioned = first & 0x80 != 0;
    if versioned {
        let version = reader.byte()? & 0x7f;
        if version != 0 {
            bail!("unsupported transaction version {version}");
        }
    }
    let header = Header {
        required_signatures: reader.byte()?,
        readonly_signed: reader.byte()?,
        readonly_unsigned: reader.byte()?,
    };
    let key_count = reader.compact()?;
    let mut keys = Vec::with_capacity(key_count);
    for _ in 0..key_count {
        keys.push(reader.pubkey()?);
    }
    let mut blockhash = [0u8; 32];
    blockhash.copy_from_slice(reader.take(32)?);
    let ix_count = reader.compact()?;
    let mut instructions = Vec::with_capacity(ix_count);
    for _ in 0..ix_count {
        let program_index = reader.byte()?;
        let accounts = reader.bytes()?;
        let data = reader.bytes()?;
        instructions.push(CompiledInstruction {
            program_index,
            accounts,
            data,
        });
    }
    let mut lookups = Vec::new();
    if versioned {
        let lookup_count = reader.compact()?;
        for _ in 0..lookup_count {
            let table = reader.pubkey()?;
            let writable = reader.bytes()?;
            let readonly = reader.bytes()?;
            lookups.push(Lookup {
                table,
                writable,
                readonly,
            });
        }
    }
    Ok(Message {
        versioned,
        header,
        keys,
        blockhash,
        instructions,
        lookups,
    })
}
