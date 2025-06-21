use soroban_sdk::{ Address, Env, Symbol };
use utils::state::oracle_registry::{ MutableOracleInfo, NormalAction, OracleInfo, OraclePriceData };

use crate::storage_types::{ HistoricalOracleData, OracleGuardRails };

pub trait OracleRegistryTrait {
    fn initialize(e: Env, admin: Address, emergency_admin: Address);

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Get the oracle price
    fn get_price(e: Env, asset_id: Symbol, cached: bool, action: NormalAction) -> OraclePriceData;

    // Get the historical oracle info
    fn get_last_price(e: Env, asset_id: Symbol) -> HistoricalOracleData;

    // Get the registered oracle info
    fn get_oracle(e: Env, asset_id: Symbol) -> OracleInfo;

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_oracle_guard_rails(e: Env) -> OracleGuardRails;
}

pub trait AdminInterface {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Create a new oracle
    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle_addr: Address,
        asset: Address,
        decimals: u32,
        sanitize_clamp_denominator: i64
    ) -> OracleInfo;

    // Update oracle info
    fn update_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        params: MutableOracleInfo
    ) -> OracleInfo;

    // Admin failsafe to manually set the oracle price
    fn set_oracle_price(e: Env, admin: Address, asset_id: Symbol, price: u128);

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_oracle_guard_rails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);
}
