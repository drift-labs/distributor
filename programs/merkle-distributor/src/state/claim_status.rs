use anchor_lang::prelude::*;

use crate::error::ErrorCode::{ArithmeticError, ClaimingIsNotStarted};

pub const START_CLAIM_PCT: u128 = 700_000; // .7
pub const PCT_PRECISION: u128 = 1_000_000;

/// Holds whether or not a claimant has claimed tokens.
#[account]
#[derive(Default)]
pub struct ClaimStatus {
    /// Authority that claimed the tokens.
    pub claimant: Pubkey,
    /// Locked amount  
    pub locked_amount: u64,
    /// Locked amount withdrawn
    pub locked_amount_withdrawn: u64,
    /// Unlocked amount
    pub unlocked_amount: u64,
    /// Unlocked amount claimed
    pub unlocked_amount_claimed: u64,
    /// indicate that whether admin can close this account, for testing purpose
    pub closable: bool,
    /// admin of merkle tree, store for for testing purpose
    pub distributor: Pubkey,
}

impl ClaimStatus {
    pub const LEN: usize = 8 + std::mem::size_of::<ClaimStatus>();

    /// Returns amount withdrawable, factoring in unlocked tokens and previous withdraws.
    /// payout is difference between the amount unlocked and the amount withdrawn
    #[allow(clippy::result_large_err)]
    pub fn amount_withdrawable(&self, curr_ts: i64, start_ts: i64, end_ts: i64) -> Result<u64> {
        let amount = self
            .unlocked_amount(curr_ts, start_ts, end_ts)?
            .checked_sub(self.locked_amount_withdrawn)
            .ok_or(ArithmeticError)?;

        Ok(amount)
    }

    /// Total amount unlocked
    /// Equal to (time_into_unlock / total_unlock_time) * locked_amount  
    /// Multiplication safety:
    ///    The maximum possible product is (2^64 -1) * (2^64 -1) = 2^128 - 2^65 + 1
    ///    which is less than 2^128 - 1 (the maximum value of a u128), meaning that
    ///    the multiplication will never overflow
    /// Truncation from u128 to u64:
    ///     Casting a u128 to a u64 will truncate the 64 higher order bits, which rounds
    ///     down from the user.
    ///     in order to avoid truncation, the final result must be less than 2^64 - 1.
    ///     Rewriting the terms, we get (time_into_unlock * locked_amount) / total_unlock_time < 2^64 - 1
    ///     We know time_into_unlock and total_unlock_time are both approximately the same size, so we can
    ///     approximate the above as:
    ///         b < 2^64 -1.
    ///     Since b is a i64, this is always true, so no truncation can occur
    #[allow(clippy::result_large_err)]
    pub fn unlocked_amount(&self, curr_ts: i64, start_ts: i64, end_ts: i64) -> Result<u64> {
        if curr_ts >= start_ts {
            if curr_ts >= end_ts {
                Ok(self.locked_amount)
            } else {
                let time_into_unlock = curr_ts.checked_sub(start_ts).ok_or(ArithmeticError)?;
                let total_unlock_time = end_ts.checked_sub(start_ts).ok_or(ArithmeticError)?;

                let amount = ((time_into_unlock as u128)
                    .checked_mul(self.locked_amount as u128)
                    .ok_or(ArithmeticError)?)
                .checked_div(total_unlock_time as u128)
                .ok_or(ArithmeticError)? as u64;

                Ok(amount)
            }
        } else {
            Ok(0)
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn update_unlocked_amount_claimed(
        &mut self,
        curr_ts: i64,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<()> {
        if curr_ts >= start_ts {
            if curr_ts >= end_ts {
                self.unlocked_amount_claimed = self.unlocked_amount;
            } else {
                let time_into_unlock = curr_ts.checked_sub(start_ts).ok_or(ArithmeticError)?;
                let total_unlock_time = end_ts.checked_sub(start_ts).ok_or(ArithmeticError)?;

                let start_amount = (self.unlocked_amount as u128)
                    .checked_mul(START_CLAIM_PCT)
                    .ok_or(ArithmeticError)?
                    .checked_div(PCT_PRECISION)
                    .ok_or(ArithmeticError)? as u64;

                let bonus_amount = ((time_into_unlock as u128)
                    .checked_mul(
                        self.unlocked_amount
                            .checked_sub(start_amount)
                            .ok_or(ArithmeticError)? as u128,
                    )
                    .ok_or(ArithmeticError)?)
                .checked_div(total_unlock_time as u128)
                .ok_or(ArithmeticError)? as u64;

                self.unlocked_amount_claimed = start_amount
                    .checked_add(bonus_amount)
                    .ok_or(ArithmeticError)?;
            }
        } else {
            return Err(ClaimingIsNotStarted.into());
        }

        Ok(())
    }

    pub fn get_unlocked_amount_forgone(&self) -> Result<u64> {
        Ok(self
            .unlocked_amount
            .checked_sub(self.unlocked_amount_claimed)
            .ok_or(ArithmeticError)?)
    }
}


#[cfg(test)]
mod test {
    use crate::error::ErrorCode::ClaimingIsNotStarted;
    use crate::state::claim_status::ClaimStatus;

    #[test]
    fn update_unlocked_amount_claimed() {
        let mut claim_status = ClaimStatus {
            unlocked_amount: 1_000_000,
          ..ClaimStatus::default()
        };

        let current_ts = 0;
        let start_ts = 1;
        let end_ts = 10;

        let result = claim_status.update_unlocked_amount_claimed(current_ts, start_ts, end_ts);

        assert_eq!(result, Err(ClaimingIsNotStarted.into()));

        let mut claim_status = ClaimStatus {
            unlocked_amount: 1_000_000,
            ..ClaimStatus::default()
        };

        let current_ts = 1;
        let start_ts = 1;
        let end_ts = 11;

        let result = claim_status.update_unlocked_amount_claimed(current_ts, start_ts, end_ts).unwrap();

        assert_eq!(claim_status.unlocked_amount_claimed, 700_000);
        assert_eq!(claim_status.get_unlocked_amount_forgone(), Ok(300_000));

        let current_ts = 6;
        let start_ts = 1;
        let end_ts = 11;

        let result = claim_status.update_unlocked_amount_claimed(current_ts, start_ts, end_ts).unwrap();

        assert_eq!(claim_status.unlocked_amount_claimed, 850_000);
        assert_eq!(claim_status.get_unlocked_amount_forgone(), Ok(150_000));

        let current_ts = 11;
        let start_ts = 1;
        let end_ts = 11;

        let result = claim_status.update_unlocked_amount_claimed(current_ts, start_ts, end_ts).unwrap();

        assert_eq!(claim_status.unlocked_amount_claimed, 1_000_000);
        assert_eq!(claim_status.get_unlocked_amount_forgone(), Ok(0));

        let current_ts = 12;
        let start_ts = 1;
        let end_ts = 11;

        let result = claim_status.update_unlocked_amount_claimed(current_ts, start_ts, end_ts).unwrap();

        assert_eq!(claim_status.unlocked_amount_claimed, 1_000_000);
        assert_eq!(claim_status.get_unlocked_amount_forgone(), Ok(0));
    }
}