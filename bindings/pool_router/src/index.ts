import { Buffer } from "buffer";
import { Address } from '@stellar/stellar-sdk';
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from '@stellar/stellar-sdk/contract';
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Typepoint,
  Duration,
} from '@stellar/stellar-sdk/contract';
export * from '@stellar/stellar-sdk'
export * as contract from '@stellar/stellar-sdk/contract'
export * as rpc from '@stellar/stellar-sdk/rpc'

if (typeof window !== 'undefined') {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}




export const PoolRouterError = {
  /**
   * PoolRouterError: PoolNotFound
   */
  301: {message:"PoolNotFound"},
  302: {message:"BadFee"},
  307: {message:"PathIsEmpty"},
  308: {message:"TokensAreNotForReward"},
  309: {message:"LiquidityNotFilled"},
  310: {message:"LiquidityAlreadyFilled"},
  312: {message:"LiquidityCalculationError"},
  313: {message:"RewardsNotConfigured"},
  314: {message:"RewardsAlreadyConfigured"},
  315: {message:"DuplicatesNotAllowed"},
  2002: {message:"TokensNotSorted"},
  2020: {message:"InMaxNotSatisfied"}
}


export interface GlobalRewardsConfig {
  expired_at: u64;
  tps: u128;
}


export interface PoolRewardInfo {
  processed: boolean;
  total_liquidity: u256;
}

export const PoolError = {
  /**
   * PoolError
   */
  401: {message:"PoolAlreadyExists"},
  404: {message:"PoolNotFound"}
}

export const AccessControlError = {
  /**
   * AccessControlError
   */
  101: {message:"RoleNotFound"},
  102: {message:"Unauthorized"},
  103: {message:"AdminAlreadySet"},
  104: {message:"BadRoleUsage"},
  2906: {message:"AnotherActionActive"},
  2907: {message:"NoActionActive"},
  2908: {message:"ActionNotReadyYet"}
}

export const RewardsError = {
  701: {message:"PastTimeNotAllowed"},
  702: {message:"SameRewardsConfig"}
}


export interface PoolRewardConfig {
  expired_at: u64;
  tps: u128;
}


export interface PoolRewardData {
  accumulated: u128;
  block: u64;
  claimed: u128;
  last_time: u64;
}


export interface UserRewardData {
  last_block: u64;
  pool_accumulated: u128;
  to_claim: u128;
}

export const UpgradeError = {
  /**
   * UpgradeError
   */
  2906: {message:"AnotherActionActive"},
  2907: {message:"NoActionActive"},
  2908: {message:"ActionNotReadyYet"}
}

export const MathError = {
  /**
   * MathError: NumberOverflow
   */
  510: {message:"NumberOverflow"},
  511: {message:"MathError"}
}

export const OracleError = {
  /**
   * OracleError: OracleNonPositive
   */
  601: {message:"OracleNonPositive"},
  602: {message:"OracleTooVolatile"},
  603: {message:"OracleStaleForPool"}
}

export const StorageError = {
  /**
   * StorageError
   */
  501: {message:"ValueNotInitialized"},
  502: {message:"ValueMissing"}
}

export const ValidationError = {
  /**
   * ValidationError
   */
  801: {message:"InvalidToken"},
  802: {message:"InvalidPercentage"},
  803: {message:"Reentrancy"},
  804: {message:"ZeroAmount"}
}


export interface PrivilegedAddresses {
  emergency_admin: string;
  emergency_pause_admins: Array<string>;
  operations_admin: string;
  pause_admin: string;
  rewards_admin: string;
}


export interface OraclePriceData {
  delay: Delay;
  price: u128;
}


export interface OracleInfo {
  address: string;
  decimals: u32;
  frozen: boolean;
  last_updated: u64;
  sanitize_clamp_denominator: u64;
}


export interface MutableOracleInfo {
  address: Option<string>;
  decimals: Option<u32>;
  frozen: Option<boolean>;
  sanitize_clamp_denominator: Option<u64>;
}

export type NormalAction = {tag: "PoolInit", values: void} | {tag: "AddLiquidity", values: void} | {tag: "RemoveLiquidity", values: void} | {tag: "Swap", values: void} | {tag: "UpdateTwap", values: void} | {tag: "Rebalance", values: void} | {tag: "ClaimInsurance", values: void};


export interface PriceDivergenceGuardRails {
  oracle_twap_percent_divergence: u64;
}


export interface ValidityGuardRails {
  seconds_before_stale_for_pool: u64;
  too_volatile_ratio: u64;
}


export interface OracleGuardRails {
  price_divergence: PriceDivergenceGuardRails;
  validity: ValidityGuardRails;
}

export type OracleValidity = {tag: "NonPositive", values: void} | {tag: "TooVolatile", values: void} | {tag: "StaleForPool", values: void} | {tag: "Frozen", values: void} | {tag: "Valid", values: void};


export interface HistoricalOracleData {
  last_oracle_price: u128;
  last_oracle_price_twap: u128;
  last_oracle_price_twap_ts: u64;
}

export type PoolStatus = {tag: "Initialized", values: void} | {tag: "Active", values: void} | {tag: "Frozen", values: void} | {tag: "ReduceOnly", values: void} | {tag: "Settlement", values: void} | {tag: "Delisted", values: void};

export type PoolTier = {tag: "A", values: void} | {tag: "B", values: void} | {tag: "C", values: void} | {tag: "Speculative", values: void} | {tag: "HighlySpeculative", values: void} | {tag: "Isolated", values: void};


export interface InsuranceClaim {
  last_revenue_withdraw_ts: u64;
  max_insurance: u128;
  rev_withdraw_since_last_settle: u128;
  settled_insurance: u128;
}


export interface PoolConfig {
  admin: string;
  assets: readonly [string, string];
  fee_fraction: u32;
  max_insurance: u128;
  oracle_registry: string;
  privileged_addrs: PrivilegedAddresses;
  protocol_fee_fraction: u32;
  router: string;
  share_token_info: TokenInitInfo;
  status: PoolStatus;
  tier: PoolTier;
  token_a_sac_address: string;
  token_b: string;
}


export interface PoolDetails {
  assets: readonly [string, string];
  fee_fraction: u32;
  insurance: InsuranceClaim;
  protocol_fee_fraction: u32;
  status: PoolStatus;
  tier: PoolTier;
}


export interface PoolResponse {
  pool: PoolDetails;
  token_a: AddressAndAmount;
  token_b: AddressAndAmount;
  token_share: AddressAndAmount;
}


export interface PoolInfo {
  pool_address: string;
  pool_response: PoolResponse;
}


export interface RewardConfig {
  reward_token: string;
}


export interface InitializeAllParams {
  config: PoolConfig;
  plane: string;
  reward_config: RewardConfig;
}

export type SwapDirection = {tag: "Buy", values: void} | {tag: "Sell", values: void};


export interface TokenInitInfo {
  name: string;
  symbol: string;
  token_wasm_hash: Buffer;
}


export interface AddressAndAmount {
  address: string;
  amount: u128;
}

export type Delay = readonly [u64];

export interface Client {
  /**
   * Construct and simulate a deposit transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  deposit: ({user, asset, desired_amount, min_shares}: {user: string, asset: string, desired_amount: u128, min_shares: u128}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<readonly [u128, u128]>>

  /**
   * Construct and simulate a swap transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  swap: ({user, asset, direction, in_amount, out_min}: {user: string, asset: string, direction: SwapDirection, in_amount: u128, out_min: u128}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a estimate_swap transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  estimate_swap: ({asset, direction, in_amount}: {asset: string, direction: SwapDirection, in_amount: u128}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<readonly [u128, i128]>>

  /**
   * Construct and simulate a withdraw transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  withdraw: ({user, asset, share_amount, min_amounts}: {user: string, asset: string, share_amount: u128, min_amounts: Array<u128>}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_privileged_addrs transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_privileged_addrs: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Map<string, Array<string>>>>

  /**
   * Construct and simulate a get_info transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_info: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Map<string, any>>>

  /**
   * Construct and simulate a get_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_pool: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a share_id transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  share_id: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a get_total_shares transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_shares: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_reserves transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_reserves: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Array<u128>>>

  /**
   * Construct and simulate a get_fee_fraction transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_fee_fraction: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a get_insurance_coverage transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_insurance_coverage: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_liquidity transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_liquidity: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u256>>

  /**
   * Construct and simulate a get_liquidity_calculator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_liquidity_calculator: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  version: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a commit_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  commit_upgrade: ({admin, new_wasm_hash}: {admin: string, new_wasm_hash: Buffer}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a apply_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  apply_upgrade: ({admin}: {admin: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Buffer>>

  /**
   * Construct and simulate a revert_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  revert_upgrade: ({admin}: {admin: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_emergency_mode transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_emergency_mode: ({emergency_admin, value}: {emergency_admin: string, value: boolean}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_emergency_mode transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_emergency_mode: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a init_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  init_admin: ({account}: {account: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_privileged_addrs transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_privileged_addrs: ({admin, rewards_admin, operations_admin, pause_admin, emergency_pause_admins}: {admin: string, rewards_admin: string, operations_admin: string, pause_admin: string, emergency_pause_admins: Array<string>}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_insurance_fund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_insurance_fund: ({admin, insurance_fund}: {admin: string, insurance_fund: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_liquidity_calculator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_liquidity_calculator: ({admin, calculator}: {admin: string, calculator: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_oracle_registry transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_oracle_registry: ({admin, oracle_registry}: {admin: string, oracle_registry: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_token_share_hash transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_token_share_hash: ({admin, new_hash}: {admin: string, new_hash: Buffer}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_pool_hash transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_pool_hash: ({admin, new_hash}: {admin: string, new_hash: Buffer}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_reward_token transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_reward_token: ({admin, reward_token}: {admin: string, reward_token: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_insurance_fund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_insurance_fund: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a get_rewards_config transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_rewards_config: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Map<string, i128>>>

  /**
   * Construct and simulate a get_tokens_for_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_tokens_for_reward: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Map<string, readonly [boolean, u256]>>>

  /**
   * Construct and simulate a get_total_liquidity transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_liquidity: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u256>>

  /**
   * Construct and simulate a config_global_rewards transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  config_global_rewards: ({user, reward_tps, expired_at, assets}: {user: string, reward_tps: u128, expired_at: u64, assets: Array<string>}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a fill_liquidity transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  fill_liquidity: ({user, asset}: {user: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a config_pool_rewards transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  config_pool_rewards: ({user, asset}: {user: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_rewards_info transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_rewards_info: ({user, asset}: {user: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Map<string, i128>>>

  /**
   * Construct and simulate a get_user_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_user_reward: ({user, asset}: {user: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_total_accumulated_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_accumulated_reward: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_total_configured_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_configured_reward: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_total_claimed_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_claimed_reward: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a get_total_outstanding_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_outstanding_reward: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a distribute_outstanding_reward transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  distribute_outstanding_reward: ({user, from, asset}: {user: string, from: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a claim transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  claim: ({user, asset}: {user: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<u128>>

  /**
   * Construct and simulate a init_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  init_pool: ({admin, assets, token_b, token_a_sac_address, share_token_info, fee_fraction, tier, max_insurance}: {admin: string, assets: readonly [string, string], token_b: string, token_a_sac_address: string, share_token_info: readonly [string, string], fee_fraction: u32, tier: PoolTier, max_insurance: u128}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a delist_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  delist_pool: ({admin, asset}: {admin: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a remove_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  remove_pool: ({admin, asset}: {admin: string, asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a query_pool_details transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  query_pool_details: ({asset}: {asset: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<PoolInfo>>

  /**
   * Construct and simulate a query_all_pools_details transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  query_all_pools_details: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Array<PoolInfo>>>

  /**
   * Construct and simulate a get_pools transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_pools: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<Array<string>>>

  /**
   * Construct and simulate a get_total_liquidity_imbalance transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_total_liquidity_imbalance: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a set_pools_plane transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_pools_plane: ({admin, plane}: {admin: string, plane: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_plane transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_plane: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a commit_transfer_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  commit_transfer_ownership: ({admin, role_name, new_address}: {admin: string, role_name: string, new_address: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a apply_transfer_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  apply_transfer_ownership: ({admin, role_name}: {admin: string, role_name: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a revert_transfer_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  revert_transfer_ownership: ({admin, role_name}: {admin: string, role_name: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_future_address transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_future_address: ({role_name}: {role_name: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAAAAAAAAAAAAAHZGVwb3NpdAAAAAAEAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAAAAAA5kZXNpcmVkX2Ftb3VudAAAAAAACgAAAAAAAAAKbWluX3NoYXJlcwAAAAAACgAAAAEAAAPtAAAAAgAAAAoAAAAK",
        "AAAAAAAAAAAAAAAEc3dhcAAAAAUAAAAAAAAABHVzZXIAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAAAAAAACWRpcmVjdGlvbgAAAAAAB9AAAAANU3dhcERpcmVjdGlvbgAAAAAAAAAAAAAJaW5fYW1vdW50AAAAAAAACgAAAAAAAAAHb3V0X21pbgAAAAAKAAAAAQAAAAo=",
        "AAAAAAAAAAAAAAANZXN0aW1hdGVfc3dhcAAAAAAAAAMAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAAAAAAJZGlyZWN0aW9uAAAAAAAH0AAAAA1Td2FwRGlyZWN0aW9uAAAAAAAAAAAAAAlpbl9hbW91bnQAAAAAAAAKAAAAAQAAA+0AAAACAAAACgAAAAs=",
        "AAAAAAAAAAAAAAAId2l0aGRyYXcAAAAEAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAAAAAAxzaGFyZV9hbW91bnQAAAAKAAAAAAAAAAttaW5fYW1vdW50cwAAAAPqAAAACgAAAAEAAAAK",
        "AAAAAAAAAAAAAAAUZ2V0X3ByaXZpbGVnZWRfYWRkcnMAAAAAAAAAAQAAA+wAAAARAAAD6gAAABM=",
        "AAAAAAAAAAAAAAAIZ2V0X2luZm8AAAABAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAD7AAAABEAAAAA",
        "AAAAAAAAAAAAAAAIZ2V0X3Bvb2wAAAABAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAAEw==",
        "AAAAAAAAAAAAAAAIc2hhcmVfaWQAAAABAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAAEw==",
        "AAAAAAAAAAAAAAAQZ2V0X3RvdGFsX3NoYXJlcwAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAK",
        "AAAAAAAAAAAAAAAMZ2V0X3Jlc2VydmVzAAAAAQAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAQAAA+oAAAAK",
        "AAAAAAAAAAAAAAAQZ2V0X2ZlZV9mcmFjdGlvbgAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAE",
        "AAAAAAAAAAAAAAAWZ2V0X2luc3VyYW5jZV9jb3ZlcmFnZQAAAAAAAQAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAQAAAAo=",
        "AAAAAAAAAAAAAAANZ2V0X2xpcXVpZGl0eQAAAAAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAM",
        "AAAAAAAAAAAAAAAYZ2V0X2xpcXVpZGl0eV9jYWxjdWxhdG9yAAAAAAAAAAEAAAAT",
        "AAAAAAAAAAAAAAAHdmVyc2lvbgAAAAAAAAAAAQAAAAQ=",
        "AAAAAAAAAAAAAAAOY29tbWl0X3VwZ3JhZGUAAAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAANbmV3X3dhc21faGFzaAAAAAAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAAAAAAANYXBwbHlfdXBncmFkZQAAAAAAAAEAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAPuAAAAIA==",
        "AAAAAAAAAAAAAAAOcmV2ZXJ0X3VwZ3JhZGUAAAAAAAEAAAAAAAAABWFkbWluAAAAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAASc2V0X2VtZXJnZW5jeV9tb2RlAAAAAAACAAAAAAAAAA9lbWVyZ2VuY3lfYWRtaW4AAAAAEwAAAAAAAAAFdmFsdWUAAAAAAAABAAAAAA==",
        "AAAAAAAAAAAAAAASZ2V0X2VtZXJnZW5jeV9tb2RlAAAAAAAAAAAAAQAAAAE=",
        "AAAAAAAAAAAAAAAKaW5pdF9hZG1pbgAAAAAAAQAAAAAAAAAHYWNjb3VudAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAUc2V0X3ByaXZpbGVnZWRfYWRkcnMAAAAFAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAADXJld2FyZHNfYWRtaW4AAAAAAAATAAAAAAAAABBvcGVyYXRpb25zX2FkbWluAAAAEwAAAAAAAAALcGF1c2VfYWRtaW4AAAAAEwAAAAAAAAAWZW1lcmdlbmN5X3BhdXNlX2FkbWlucwAAAAAD6gAAABMAAAAA",
        "AAAAAAAAAAAAAAASc2V0X2luc3VyYW5jZV9mdW5kAAAAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAADmluc3VyYW5jZV9mdW5kAAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAYc2V0X2xpcXVpZGl0eV9jYWxjdWxhdG9yAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAApjYWxjdWxhdG9yAAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAATc2V0X29yYWNsZV9yZWdpc3RyeQAAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAAD29yYWNsZV9yZWdpc3RyeQAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAUc2V0X3Rva2VuX3NoYXJlX2hhc2gAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAACG5ld19oYXNoAAAD7gAAACAAAAAA",
        "AAAAAAAAAAAAAAANc2V0X3Bvb2xfaGFzaAAAAAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAIbmV3X2hhc2gAAAPuAAAAIAAAAAA=",
        "AAAAAAAAAAAAAAAQc2V0X3Jld2FyZF90b2tlbgAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAMcmV3YXJkX3Rva2VuAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAASZ2V0X2luc3VyYW5jZV9mdW5kAAAAAAAAAAAAAQAAABM=",
        "AAAAAAAAAAAAAAASZ2V0X3Jld2FyZHNfY29uZmlnAAAAAAAAAAAAAQAAA+wAAAARAAAACw==",
        "AAAAAAAAAAAAAAAVZ2V0X3Rva2Vuc19mb3JfcmV3YXJkAAAAAAAAAAAAAAEAAAPsAAAAEQAAA+0AAAACAAAAAQAAAAw=",
        "AAAAAAAAAAAAAAATZ2V0X3RvdGFsX2xpcXVpZGl0eQAAAAABAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAADA==",
        "AAAAAAAAAAAAAAAVY29uZmlnX2dsb2JhbF9yZXdhcmRzAAAAAAAABAAAAAAAAAAEdXNlcgAAABMAAAAAAAAACnJld2FyZF90cHMAAAAAAAoAAAAAAAAACmV4cGlyZWRfYXQAAAAAAAYAAAAAAAAABmFzc2V0cwAAAAAD6gAAABEAAAAA",
        "AAAAAAAAAAAAAAAOZmlsbF9saXF1aWRpdHkAAAAAAAIAAAAAAAAABHVzZXIAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAATY29uZmlnX3Bvb2xfcmV3YXJkcwAAAAACAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAQAAAAo=",
        "AAAAAAAAAAAAAAAQZ2V0X3Jld2FyZHNfaW5mbwAAAAIAAAAAAAAABHVzZXIAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAD7AAAABEAAAAL",
        "AAAAAAAAAAAAAAAPZ2V0X3VzZXJfcmV3YXJkAAAAAAIAAAAAAAAABHVzZXIAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAACg==",
        "AAAAAAAAAAAAAAAcZ2V0X3RvdGFsX2FjY3VtdWxhdGVkX3Jld2FyZAAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAK",
        "AAAAAAAAAAAAAAAbZ2V0X3RvdGFsX2NvbmZpZ3VyZWRfcmV3YXJkAAAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAK",
        "AAAAAAAAAAAAAAAYZ2V0X3RvdGFsX2NsYWltZWRfcmV3YXJkAAAAAQAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAQAAAAo=",
        "AAAAAAAAAAAAAAAcZ2V0X3RvdGFsX291dHN0YW5kaW5nX3Jld2FyZAAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAK",
        "AAAAAAAAAAAAAAAdZGlzdHJpYnV0ZV9vdXRzdGFuZGluZ19yZXdhcmQAAAAAAAADAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEZnJvbQAAABMAAAAAAAAABWFzc2V0AAAAAAAAEQAAAAEAAAAK",
        "AAAAAAAAAAAAAAAFY2xhaW0AAAAAAAACAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAFYXNzZXQAAAAAAAARAAAAAQAAAAo=",
        "AAAAAAAAAAAAAAAJaW5pdF9wb29sAAAAAAAACAAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAZhc3NldHMAAAAAA+0AAAACAAAAEQAAABEAAAAAAAAAB3Rva2VuX2IAAAAAEwAAAAAAAAATdG9rZW5fYV9zYWNfYWRkcmVzcwAAAAATAAAAAAAAABBzaGFyZV90b2tlbl9pbmZvAAAD7QAAAAIAAAAQAAAAEAAAAAAAAAAMZmVlX2ZyYWN0aW9uAAAABAAAAAAAAAAEdGllcgAAB9AAAAAIUG9vbFRpZXIAAAAAAAAADW1heF9pbnN1cmFuY2UAAAAAAAAKAAAAAQAAABM=",
        "AAAAAAAAAAAAAAALZGVsaXN0X3Bvb2wAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAALcmVtb3ZlX3Bvb2wAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAVhc3NldAAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAAScXVlcnlfcG9vbF9kZXRhaWxzAAAAAAABAAAAAAAAAAVhc3NldAAAAAAAABEAAAABAAAH0AAAAAhQb29sSW5mbw==",
        "AAAAAAAAAAAAAAAXcXVlcnlfYWxsX3Bvb2xzX2RldGFpbHMAAAAAAAAAAAEAAAPqAAAH0AAAAAhQb29sSW5mbw==",
        "AAAAAAAAAAAAAAAJZ2V0X3Bvb2xzAAAAAAAAAAAAAAEAAAPqAAAAEw==",
        "AAAAAAAAAAAAAAAdZ2V0X3RvdGFsX2xpcXVpZGl0eV9pbWJhbGFuY2UAAAAAAAAAAAAAAQAAAAs=",
        "AAAAAAAAAAAAAAAPc2V0X3Bvb2xzX3BsYW5lAAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAFcGxhbmUAAAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAJZ2V0X3BsYW5lAAAAAAAAAAAAAAEAAAAT",
        "AAAAAAAAAAAAAAAZY29tbWl0X3RyYW5zZmVyX293bmVyc2hpcAAAAAAAAAMAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAJcm9sZV9uYW1lAAAAAAAAEQAAAAAAAAALbmV3X2FkZHJlc3MAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAAYYXBwbHlfdHJhbnNmZXJfb3duZXJzaGlwAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAlyb2xlX25hbWUAAAAAAAARAAAAAA==",
        "AAAAAAAAAAAAAAAZcmV2ZXJ0X3RyYW5zZmVyX293bmVyc2hpcAAAAAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAJcm9sZV9uYW1lAAAAAAAAEQAAAAA=",
        "AAAAAAAAAAAAAAASZ2V0X2Z1dHVyZV9hZGRyZXNzAAAAAAABAAAAAAAAAAlyb2xlX25hbWUAAAAAAAARAAAAAQAAABM=",
        "AAAABAAAAAAAAAAAAAAAD1Bvb2xSb3V0ZXJFcnJvcgAAAAAMAAAAHVBvb2xSb3V0ZXJFcnJvcjogUG9vbE5vdEZvdW5kAAAAAAAADFBvb2xOb3RGb3VuZAAAAS0AAAAAAAAABkJhZEZlZQAAAAABLgAAAAAAAAALUGF0aElzRW1wdHkAAAABMwAAAAAAAAAVVG9rZW5zQXJlTm90Rm9yUmV3YXJkAAAAAAABNAAAAAAAAAASTGlxdWlkaXR5Tm90RmlsbGVkAAAAAAE1AAAAAAAAABZMaXF1aWRpdHlBbHJlYWR5RmlsbGVkAAAAAAE2AAAAAAAAABlMaXF1aWRpdHlDYWxjdWxhdGlvbkVycm9yAAAAAAABOAAAAAAAAAAUUmV3YXJkc05vdENvbmZpZ3VyZWQAAAE5AAAAAAAAABhSZXdhcmRzQWxyZWFkeUNvbmZpZ3VyZWQAAAE6AAAAAAAAABREdXBsaWNhdGVzTm90QWxsb3dlZAAAATsAAAAAAAAAD1Rva2Vuc05vdFNvcnRlZAAAAAfSAAAAAAAAABFJbk1heE5vdFNhdGlzZmllZAAAAAAAB+Q=",
        "AAAAAQAAAAAAAAAAAAAAE0dsb2JhbFJld2FyZHNDb25maWcAAAAAAgAAAAAAAAAKZXhwaXJlZF9hdAAAAAAABgAAAAAAAAADdHBzAAAAAAo=",
        "AAAAAQAAAAAAAAAAAAAADlBvb2xSZXdhcmRJbmZvAAAAAAACAAAAAAAAAAlwcm9jZXNzZWQAAAAAAAABAAAAAAAAAA90b3RhbF9saXF1aWRpdHkAAAAADA==",
        "AAAABAAAAAAAAAAAAAAACVBvb2xFcnJvcgAAAAAAAAIAAAAJUG9vbEVycm9yAAAAAAAAEVBvb2xBbHJlYWR5RXhpc3RzAAAAAAABkQAAAAAAAAAMUG9vbE5vdEZvdW5kAAABlA==",
        "AAAABAAAAAAAAAAAAAAAEkFjY2Vzc0NvbnRyb2xFcnJvcgAAAAAABwAAABJBY2Nlc3NDb250cm9sRXJyb3IAAAAAAAxSb2xlTm90Rm91bmQAAABlAAAAAAAAAAxVbmF1dGhvcml6ZWQAAABmAAAAAAAAAA9BZG1pbkFscmVhZHlTZXQAAAAAZwAAAAAAAAAMQmFkUm9sZVVzYWdlAAAAaAAAAAAAAAATQW5vdGhlckFjdGlvbkFjdGl2ZQAAAAtaAAAAAAAAAA5Ob0FjdGlvbkFjdGl2ZQAAAAALWwAAAAAAAAARQWN0aW9uTm90UmVhZHlZZXQAAAAAAAtc",
        "AAAABAAAAAAAAAAAAAAADFJld2FyZHNFcnJvcgAAAAIAAAAAAAAAElBhc3RUaW1lTm90QWxsb3dlZAAAAAACvQAAAAAAAAARU2FtZVJld2FyZHNDb25maWcAAAAAAAK+",
        "AAAAAQAAAAAAAAAAAAAAEFBvb2xSZXdhcmRDb25maWcAAAACAAAAAAAAAApleHBpcmVkX2F0AAAAAAAGAAAAAAAAAAN0cHMAAAAACg==",
        "AAAAAQAAAAAAAAAAAAAADlBvb2xSZXdhcmREYXRhAAAAAAAEAAAAAAAAAAthY2N1bXVsYXRlZAAAAAAKAAAAAAAAAAVibG9jawAAAAAAAAYAAAAAAAAAB2NsYWltZWQAAAAACgAAAAAAAAAJbGFzdF90aW1lAAAAAAAABg==",
        "AAAAAQAAAAAAAAAAAAAADlVzZXJSZXdhcmREYXRhAAAAAAADAAAAAAAAAApsYXN0X2Jsb2NrAAAAAAAGAAAAAAAAABBwb29sX2FjY3VtdWxhdGVkAAAACgAAAAAAAAAIdG9fY2xhaW0AAAAK",
        "AAAABAAAAAAAAAAAAAAADFVwZ3JhZGVFcnJvcgAAAAMAAAAMVXBncmFkZUVycm9yAAAAE0Fub3RoZXJBY3Rpb25BY3RpdmUAAAALWgAAAAAAAAAOTm9BY3Rpb25BY3RpdmUAAAAAC1sAAAAAAAAAEUFjdGlvbk5vdFJlYWR5WWV0AAAAAAALXA==",
        "AAAABAAAAAAAAAAAAAAACU1hdGhFcnJvcgAAAAAAAAIAAAAZTWF0aEVycm9yOiBOdW1iZXJPdmVyZmxvdwAAAAAAAA5OdW1iZXJPdmVyZmxvdwAAAAAB/gAAAAAAAAAJTWF0aEVycm9yAAAAAAAB/w==",
        "AAAABAAAAAAAAAAAAAAAC09yYWNsZUVycm9yAAAAAAMAAAAeT3JhY2xlRXJyb3I6IE9yYWNsZU5vblBvc2l0aXZlAAAAAAART3JhY2xlTm9uUG9zaXRpdmUAAAAAAAJZAAAAAAAAABFPcmFjbGVUb29Wb2xhdGlsZQAAAAAAAloAAAAAAAAAEk9yYWNsZVN0YWxlRm9yUG9vbAAAAAACWw==",
        "AAAABAAAAAAAAAAAAAAADFN0b3JhZ2VFcnJvcgAAAAIAAAAMU3RvcmFnZUVycm9yAAAAE1ZhbHVlTm90SW5pdGlhbGl6ZWQAAAAB9QAAAAAAAAAMVmFsdWVNaXNzaW5nAAAB9g==",
        "AAAABAAAAAAAAAAAAAAAD1ZhbGlkYXRpb25FcnJvcgAAAAAEAAAAD1ZhbGlkYXRpb25FcnJvcgAAAAAMSW52YWxpZFRva2VuAAADIQAAAAAAAAARSW52YWxpZFBlcmNlbnRhZ2UAAAAAAAMiAAAAAAAAAApSZWVudHJhbmN5AAAAAAMjAAAAAAAAAApaZXJvQW1vdW50AAAAAAMk",
        "AAAAAQAAAAAAAAAAAAAAE1ByaXZpbGVnZWRBZGRyZXNzZXMAAAAABQAAAAAAAAAPZW1lcmdlbmN5X2FkbWluAAAAABMAAAAAAAAAFmVtZXJnZW5jeV9wYXVzZV9hZG1pbnMAAAAAA+oAAAATAAAAAAAAABBvcGVyYXRpb25zX2FkbWluAAAAEwAAAAAAAAALcGF1c2VfYWRtaW4AAAAAEwAAAAAAAAANcmV3YXJkc19hZG1pbgAAAAAAABM=",
        "AAAAAQAAAAAAAAAAAAAAD09yYWNsZVByaWNlRGF0YQAAAAACAAAAAAAAAAVkZWxheQAAAAAAB9AAAAAFRGVsYXkAAAAAAAAAAAAABXByaWNlAAAAAAAACg==",
        "AAAAAQAAAAAAAAAAAAAACk9yYWNsZUluZm8AAAAAAAUAAAAAAAAAB2FkZHJlc3MAAAAAEwAAAAAAAAAIZGVjaW1hbHMAAAAEAAAAAAAAAAZmcm96ZW4AAAAAAAEAAAAAAAAADGxhc3RfdXBkYXRlZAAAAAYAAAAAAAAAGnNhbml0aXplX2NsYW1wX2Rlbm9taW5hdG9yAAAAAAAG",
        "AAAAAQAAAAAAAAAAAAAAEU11dGFibGVPcmFjbGVJbmZvAAAAAAAABAAAAAAAAAAHYWRkcmVzcwAAAAPoAAAAEwAAAAAAAAAIZGVjaW1hbHMAAAPoAAAABAAAAAAAAAAGZnJvemVuAAAAAAPoAAAAAQAAAAAAAAAac2FuaXRpemVfY2xhbXBfZGVub21pbmF0b3IAAAAAA+gAAAAG",
        "AAAAAgAAAAAAAAAAAAAADE5vcm1hbEFjdGlvbgAAAAcAAAAAAAAAAAAAAAhQb29sSW5pdAAAAAAAAAAAAAAADEFkZExpcXVpZGl0eQAAAAAAAAAAAAAAD1JlbW92ZUxpcXVpZGl0eQAAAAAAAAAAAAAAAARTd2FwAAAAAAAAAAAAAAAKVXBkYXRlVHdhcAAAAAAAAAAAAAAAAAAJUmViYWxhbmNlAAAAAAAAAAAAAAAAAAAOQ2xhaW1JbnN1cmFuY2UAAA==",
        "AAAAAQAAAAAAAAAAAAAAGVByaWNlRGl2ZXJnZW5jZUd1YXJkUmFpbHMAAAAAAAABAAAAAAAAAB5vcmFjbGVfdHdhcF9wZXJjZW50X2RpdmVyZ2VuY2UAAAAAAAY=",
        "AAAAAQAAAAAAAAAAAAAAElZhbGlkaXR5R3VhcmRSYWlscwAAAAAAAgAAAAAAAAAdc2Vjb25kc19iZWZvcmVfc3RhbGVfZm9yX3Bvb2wAAAAAAAAGAAAAAAAAABJ0b29fdm9sYXRpbGVfcmF0aW8AAAAAAAY=",
        "AAAAAQAAAAAAAAAAAAAAEE9yYWNsZUd1YXJkUmFpbHMAAAACAAAAAAAAABBwcmljZV9kaXZlcmdlbmNlAAAH0AAAABlQcmljZURpdmVyZ2VuY2VHdWFyZFJhaWxzAAAAAAAAAAAAAAh2YWxpZGl0eQAAB9AAAAASVmFsaWRpdHlHdWFyZFJhaWxzAAA=",
        "AAAAAgAAAAAAAAAAAAAADk9yYWNsZVZhbGlkaXR5AAAAAAAFAAAAAAAAAAAAAAALTm9uUG9zaXRpdmUAAAAAAAAAAAAAAAALVG9vVm9sYXRpbGUAAAAAAAAAAAAAAAAMU3RhbGVGb3JQb29sAAAAAAAAAAAAAAAGRnJvemVuAAAAAAAAAAAAAAAAAAVWYWxpZAAAAA==",
        "AAAAAQAAAAAAAAAAAAAAFEhpc3RvcmljYWxPcmFjbGVEYXRhAAAAAwAAAAAAAAARbGFzdF9vcmFjbGVfcHJpY2UAAAAAAAAKAAAAAAAAABZsYXN0X29yYWNsZV9wcmljZV90d2FwAAAAAAAKAAAAAAAAABlsYXN0X29yYWNsZV9wcmljZV90d2FwX3RzAAAAAAAABg==",
        "AAAAAgAAAAAAAAAAAAAAClBvb2xTdGF0dXMAAAAAAAYAAAAAAAAAAAAAAAtJbml0aWFsaXplZAAAAAAAAAAAAAAAAAZBY3RpdmUAAAAAAAAAAAAAAAAABkZyb3plbgAAAAAAAAAAAAAAAAAKUmVkdWNlT25seQAAAAAAAAAAAAAAAAAKU2V0dGxlbWVudAAAAAAAAAAAAAAAAAAIRGVsaXN0ZWQ=",
        "AAAAAgAAAAAAAAAAAAAACFBvb2xUaWVyAAAABgAAAAAAAAAAAAAAAUEAAAAAAAAAAAAAAAAAAAFCAAAAAAAAAAAAAAAAAAABQwAAAAAAAAAAAAAAAAAAC1NwZWN1bGF0aXZlAAAAAAAAAAAAAAAAEUhpZ2hseVNwZWN1bGF0aXZlAAAAAAAAAAAAAAAAAAAISXNvbGF0ZWQ=",
        "AAAAAQAAAAAAAAAAAAAADkluc3VyYW5jZUNsYWltAAAAAAAEAAAAAAAAABhsYXN0X3JldmVudWVfd2l0aGRyYXdfdHMAAAAGAAAAAAAAAA1tYXhfaW5zdXJhbmNlAAAAAAAACgAAAAAAAAAecmV2X3dpdGhkcmF3X3NpbmNlX2xhc3Rfc2V0dGxlAAAAAAAKAAAAAAAAABFzZXR0bGVkX2luc3VyYW5jZQAAAAAAAAo=",
        "AAAAAQAAAAAAAAAAAAAAClBvb2xDb25maWcAAAAAAA0AAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAGYXNzZXRzAAAAAAPtAAAAAgAAABEAAAARAAAAAAAAAAxmZWVfZnJhY3Rpb24AAAAEAAAAAAAAAA1tYXhfaW5zdXJhbmNlAAAAAAAACgAAAAAAAAAPb3JhY2xlX3JlZ2lzdHJ5AAAAABMAAAAAAAAAEHByaXZpbGVnZWRfYWRkcnMAAAfQAAAAE1ByaXZpbGVnZWRBZGRyZXNzZXMAAAAAAAAAABVwcm90b2NvbF9mZWVfZnJhY3Rpb24AAAAAAAAEAAAAAAAAAAZyb3V0ZXIAAAAAABMAAAAAAAAAEHNoYXJlX3Rva2VuX2luZm8AAAfQAAAADVRva2VuSW5pdEluZm8AAAAAAAAAAAAABnN0YXR1cwAAAAAH0AAAAApQb29sU3RhdHVzAAAAAAAAAAAABHRpZXIAAAfQAAAACFBvb2xUaWVyAAAAAAAAABN0b2tlbl9hX3NhY19hZGRyZXNzAAAAABMAAAAAAAAAB3Rva2VuX2IAAAAAEw==",
        "AAAAAQAAAAAAAAAAAAAAC1Bvb2xEZXRhaWxzAAAAAAYAAAAAAAAABmFzc2V0cwAAAAAD7QAAAAIAAAARAAAAEQAAAAAAAAAMZmVlX2ZyYWN0aW9uAAAABAAAAAAAAAAJaW5zdXJhbmNlAAAAAAAH0AAAAA5JbnN1cmFuY2VDbGFpbQAAAAAAAAAAABVwcm90b2NvbF9mZWVfZnJhY3Rpb24AAAAAAAAEAAAAAAAAAAZzdGF0dXMAAAAAB9AAAAAKUG9vbFN0YXR1cwAAAAAAAAAAAAR0aWVyAAAH0AAAAAhQb29sVGllcg==",
        "AAAAAQAAAAAAAAAAAAAADFBvb2xSZXNwb25zZQAAAAQAAAAAAAAABHBvb2wAAAfQAAAAC1Bvb2xEZXRhaWxzAAAAAAAAAAAHdG9rZW5fYQAAAAfQAAAAEEFkZHJlc3NBbmRBbW91bnQAAAAAAAAAB3Rva2VuX2IAAAAH0AAAABBBZGRyZXNzQW5kQW1vdW50AAAAAAAAAAt0b2tlbl9zaGFyZQAAAAfQAAAAEEFkZHJlc3NBbmRBbW91bnQ=",
        "AAAAAQAAAAAAAAAAAAAACFBvb2xJbmZvAAAAAgAAAAAAAAAMcG9vbF9hZGRyZXNzAAAAEwAAAAAAAAANcG9vbF9yZXNwb25zZQAAAAAAB9AAAAAMUG9vbFJlc3BvbnNl",
        "AAAAAQAAAAAAAAAAAAAADFJld2FyZENvbmZpZwAAAAEAAAAAAAAADHJld2FyZF90b2tlbgAAABM=",
        "AAAAAQAAAAAAAAAAAAAAE0luaXRpYWxpemVBbGxQYXJhbXMAAAAAAwAAAAAAAAAGY29uZmlnAAAAAAfQAAAAClBvb2xDb25maWcAAAAAAAAAAAAFcGxhbmUAAAAAAAATAAAAAAAAAA1yZXdhcmRfY29uZmlnAAAAAAAH0AAAAAxSZXdhcmRDb25maWc=",
        "AAAAAgAAAAAAAAAAAAAADVN3YXBEaXJlY3Rpb24AAAAAAAACAAAAAAAAAAAAAAADQnV5AAAAAAAAAAAAAAAABFNlbGw=",
        "AAAAAQAAAAAAAAAAAAAADVRva2VuSW5pdEluZm8AAAAAAAADAAAAAAAAAARuYW1lAAAAEAAAAAAAAAAGc3ltYm9sAAAAAAAQAAAAAAAAAA90b2tlbl93YXNtX2hhc2gAAAAD7gAAACA=",
        "AAAAAQAAAAAAAAAAAAAAEEFkZHJlc3NBbmRBbW91bnQAAAACAAAAAAAAAAdhZGRyZXNzAAAAABMAAAAAAAAABmFtb3VudAAAAAAACg==",
        "AAAAAQAAAAAAAAAAAAAABURlbGF5AAAAAAAAAQAAAAAAAAABMAAAAAAAAAY=" ]),
      options
    )
  }
  public readonly fromJSON = {
    deposit: this.txFromJSON<readonly [u128, u128]>,
        swap: this.txFromJSON<u128>,
        estimate_swap: this.txFromJSON<readonly [u128, i128]>,
        withdraw: this.txFromJSON<u128>,
        get_privileged_addrs: this.txFromJSON<Map<string, Array<string>>>,
        get_info: this.txFromJSON<Map<string, any>>,
        get_pool: this.txFromJSON<string>,
        share_id: this.txFromJSON<string>,
        get_total_shares: this.txFromJSON<u128>,
        get_reserves: this.txFromJSON<Array<u128>>,
        get_fee_fraction: this.txFromJSON<u32>,
        get_insurance_coverage: this.txFromJSON<u128>,
        get_liquidity: this.txFromJSON<u256>,
        get_liquidity_calculator: this.txFromJSON<string>,
        version: this.txFromJSON<u32>,
        commit_upgrade: this.txFromJSON<null>,
        apply_upgrade: this.txFromJSON<Buffer>,
        revert_upgrade: this.txFromJSON<null>,
        set_emergency_mode: this.txFromJSON<null>,
        get_emergency_mode: this.txFromJSON<boolean>,
        init_admin: this.txFromJSON<null>,
        set_privileged_addrs: this.txFromJSON<null>,
        set_insurance_fund: this.txFromJSON<null>,
        set_liquidity_calculator: this.txFromJSON<null>,
        set_oracle_registry: this.txFromJSON<null>,
        set_token_share_hash: this.txFromJSON<null>,
        set_pool_hash: this.txFromJSON<null>,
        set_reward_token: this.txFromJSON<null>,
        get_insurance_fund: this.txFromJSON<string>,
        get_rewards_config: this.txFromJSON<Map<string, i128>>,
        get_tokens_for_reward: this.txFromJSON<Map<string, readonly [boolean, u256]>>,
        get_total_liquidity: this.txFromJSON<u256>,
        config_global_rewards: this.txFromJSON<null>,
        fill_liquidity: this.txFromJSON<null>,
        config_pool_rewards: this.txFromJSON<u128>,
        get_rewards_info: this.txFromJSON<Map<string, i128>>,
        get_user_reward: this.txFromJSON<u128>,
        get_total_accumulated_reward: this.txFromJSON<u128>,
        get_total_configured_reward: this.txFromJSON<u128>,
        get_total_claimed_reward: this.txFromJSON<u128>,
        get_total_outstanding_reward: this.txFromJSON<u128>,
        distribute_outstanding_reward: this.txFromJSON<u128>,
        claim: this.txFromJSON<u128>,
        init_pool: this.txFromJSON<string>,
        delist_pool: this.txFromJSON<null>,
        remove_pool: this.txFromJSON<null>,
        query_pool_details: this.txFromJSON<PoolInfo>,
        query_all_pools_details: this.txFromJSON<Array<PoolInfo>>,
        get_pools: this.txFromJSON<Array<string>>,
        get_total_liquidity_imbalance: this.txFromJSON<i128>,
        set_pools_plane: this.txFromJSON<null>,
        get_plane: this.txFromJSON<string>,
        commit_transfer_ownership: this.txFromJSON<null>,
        apply_transfer_ownership: this.txFromJSON<null>,
        revert_transfer_ownership: this.txFromJSON<null>,
        get_future_address: this.txFromJSON<string>
  }
}