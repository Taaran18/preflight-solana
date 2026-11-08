use crate::{
    simulate::SimOutcome,
    solana::{AccountMeta, Instruction, Pubkey, SOL_MINT},
};

const VERIFY: u8 = 0;
const TOKEN_MIN: u8 = 0;
const LAMPORTS_MIN: u8 = 2;
const TOKEN_OWNER: u8 = 4;
const MAX_CHECKS: usize = 32;

pub struct GuardContext<'a> {
    pub output_mint: Option<&'a str>,
    pub output_quoted_raw: u64,
    pub output_threshold_raw: u64,
    pub sol_slack_lamports: u64,
}

pub struct GuardPlan {
    pub instruction: Instruction,
    pub labels: Vec<String>,
}

fn amount(raw: u64, decimals: u8) -> String {
    let value = raw as f64 / 10f64.powi(decimals as i32);
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn build(program: Pubkey, owner: Pubkey, receipt: &[u8; 32], outcome: &SimOutcome, ctx: &GuardContext) -> Option<GuardPlan> {
    let post_lamports = outcome.post_lamports?;
    let mut accounts = vec![AccountMeta::readonly(owner, true)];
    let mut checks: Vec<(u8, u8, u64)> = Vec::new();
    let mut labels = Vec::new();

    let mut sol_slack = ctx.sol_slack_lamports;
    if ctx.output_mint == Some(SOL_MINT) {
        let quoted_slack = ctx.output_quoted_raw.saturating_sub(ctx.output_threshold_raw);
        let simulated_gain = post_lamports.saturating_sub(outcome.pre_lamports);
        let simulated_extra = simulated_gain.saturating_sub(ctx.output_quoted_raw);
        sol_slack = sol_slack.saturating_add(quoted_slack).saturating_add(simulated_extra);
    }
    let min_lamports = post_lamports.saturating_sub(sol_slack);
    checks.push((LAMPORTS_MIN, 0, min_lamports));
    labels.push(format!("SOL balance stays at or above {}", amount(min_lamports, 9)));

    for token in &outcome.tokens {
        let Some(post) = token.post else {
            continue;
        };
        if checks.len() + 2 > MAX_CHECKS {
            break;
        }
        let index = accounts.len() as u8;
        accounts.push(AccountMeta::readonly(token.account, false));
        let pre = token.pre.unwrap_or(0);
        let receives_output = ctx.output_mint == Some(token.mint.as_str()) && post > pre;
        let minimum = if receives_output { pre.saturating_add(ctx.output_threshold_raw).min(post) } else { post };
        let decimals = outcome.decimals.get(&token.mint).copied().unwrap_or(0);
        let symbol = outcome.symbols.get(&token.mint).cloned().unwrap_or_else(|| "token".to_string());
        checks.push((TOKEN_MIN, index, minimum));
        labels.push(format!("{symbol} balance ends at or above {}", amount(minimum, decimals)));
        checks.push((TOKEN_OWNER, index, 0));
        labels.push(format!("You stay the owner of your {symbol} account"));
    }

    let mut data = Vec::with_capacity(34 + checks.len() * 10);
    data.push(VERIFY);
    data.extend_from_slice(receipt);
    data.push(checks.len() as u8);
    for (kind, index, value) in &checks {
        data.push(*kind);
        data.push(*index);
        data.extend_from_slice(&value.to_le_bytes());
    }
    Some(GuardPlan {
        instruction: Instruction {
            program_id: program,
            accounts,
            data,
        },
        labels,
    })
}
