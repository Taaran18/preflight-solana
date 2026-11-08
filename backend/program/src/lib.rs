#![no_std]

#[cfg(test)]
extern crate std;

pub mod checks;

use checks::{hex32, lamports_check, parse, token_check, write_u32, Check, TokenView, ERROR_BASE};
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
use solana_program_log::logger::log_message;

pub const TOKEN_PROGRAM: Address = Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_PROGRAM: Address = Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

#[cfg(not(feature = "no-entrypoint"))]
mod entry {
    use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};

    program_entrypoint!(super::process_instruction);
    no_allocator!();
    nostd_panic_handler!();
}

pub fn process_instruction(_program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let parsed = parse(data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let authority = accounts.first().ok_or(ProgramError::NotEnoughAccountKeys)?;
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    for (index, check) in parsed.checks().enumerate() {
        if !run_check(&check, accounts)? {
            log_failure(index as u32);
            return Err(ProgramError::Custom(ERROR_BASE + index as u32));
        }
    }
    log_success(parsed.receipt, parsed.len() as u32);
    Ok(())
}

fn run_check(check: &Check, accounts: &[AccountView]) -> Result<bool, ProgramError> {
    let account = accounts.get(check.account as usize).ok_or(ProgramError::NotEnoughAccountKeys)?;
    if !check.kind.needs_token_account() {
        return Ok(lamports_check(check.kind, account.lamports(), check.value));
    }
    if !(account.owned_by(&TOKEN_PROGRAM) || account.owned_by(&TOKEN_2022_PROGRAM)) {
        return Ok(false);
    }
    let data = account.try_borrow()?;
    let view = TokenView::new(&data).ok_or(ProgramError::InvalidAccountData)?;
    let expected_owner = match check.kind {
        checks::CheckKind::TokenOwner => Some(
            accounts
                .get(check.value as usize)
                .ok_or(ProgramError::NotEnoughAccountKeys)?
                .address()
                .as_array()
                .as_slice(),
        ),
        _ => None,
    };
    Ok(token_check(check.kind, &view, check.value, expected_owner))
}

fn log_success(receipt: &[u8], count: u32) {
    const PREFIX: &[u8] = b"Preflight receipt ";
    const MIDDLE: &[u8] = b" verified checks=";
    let mut buffer = [0u8; 128];
    let mut cursor = 0;
    buffer[cursor..cursor + PREFIX.len()].copy_from_slice(PREFIX);
    cursor += PREFIX.len();
    let mut hex = [0u8; 64];
    hex32(receipt, &mut hex);
    buffer[cursor..cursor + 64].copy_from_slice(&hex);
    cursor += 64;
    buffer[cursor..cursor + MIDDLE.len()].copy_from_slice(MIDDLE);
    cursor += MIDDLE.len();
    let mut digits = [0u8; 10];
    let len = write_u32(count, &mut digits);
    buffer[cursor..cursor + len].copy_from_slice(&digits[..len]);
    cursor += len;
    log_message(&buffer[..cursor]);
}

fn log_failure(index: u32) {
    const PREFIX: &[u8] = b"Preflight guard check failed index=";
    let mut buffer = [0u8; 64];
    buffer[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut digits = [0u8; 10];
    let len = write_u32(index, &mut digits);
    buffer[PREFIX.len()..PREFIX.len() + len].copy_from_slice(&digits[..len]);
    log_message(&buffer[..PREFIX.len() + len]);
}
