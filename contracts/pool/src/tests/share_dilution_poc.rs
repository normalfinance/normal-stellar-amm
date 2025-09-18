#[cfg(test)]
mod tests {
    // Pure math PoC: demonstrate NAV/share underpricing when shares mint 1:1 with B
    // while NAV can increase via synthetic A mint (rebalance), and withdrawals pay B 1:1.

    #[test]
    fn share_underpricing_math_only() {
        // Initial pool state after first LP deposit and rebalance that minted A:
        // Suppose initial depositor added 10_000 B and pool minted 2_000 A via rebalance.
        let initial_reserve_b: u128 = 10_000;
        let initial_reserve_a: u128 = 2_000;

        // Total shares outstanding remain equal to initial B deposit (1:1 rule)
        let total_shares: u128 = 10_000;

        // Oracle peg price of A in B terms (e.g., 2 B per A)
        let peg_a_in_b: u128 = 2; // 1 A == 2 B

        // Compute NAV in B terms and NAV/share
        let nav_b: u128 = initial_reserve_b + initial_reserve_a * peg_a_in_b; // 10_000 + 2_000*2 = 14_000 B
        let nav_per_share_b: u128 = nav_b / total_shares; // 1.4 B per share (integer floor: 1)
        assert!(nav_per_share_b >= 1, "NAV/share should be at least 1 B after A mint");

        // Attacker deposits a small amount of B and receives shares 1:1
        let attacker_deposit_b: u128 = 100;
        let attacker_shares_minted: u128 = attacker_deposit_b; // 1:1 rule

        // After attacker deposit, recompute NAV and total shares
        let final_reserve_b: u128 = initial_reserve_b + attacker_deposit_b; // A unchanged by deposit
        let final_reserve_a: u128 = initial_reserve_a;
        let final_total_shares: u128 = total_shares + attacker_shares_minted; // 10_100
        let final_nav_b: u128 = final_reserve_b + final_reserve_a * peg_a_in_b; // 10_100 + 4_000 = 14_100

        // Attacker's claim value at NAV/share (B terms)
        let attacker_value_b_estimate: u128 = (final_nav_b * attacker_shares_minted) / final_total_shares;

        // Underpricing: claim value exceeds amount paid (attacker_deposit_b)
        assert!(
            attacker_value_b_estimate > attacker_deposit_b,
            "Attacker receives underpriced shares relative to NAV/share"
        );
        
        // With B-only withdrawals at 1:1 per share, exits are at par in B.
        // This preserves the underpricing gap for entrants and dilutes incumbents.
        let withdraw_b: u128 = attacker_shares_minted; // 1:1 on exit
        assert_eq!(withdraw_b, attacker_deposit_b);
    }
}