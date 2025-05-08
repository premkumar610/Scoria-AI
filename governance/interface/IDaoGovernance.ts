// types/IDaoGovernance.ts

import type { BigNumber, BytesLike, PopulatedTransaction, Signer } from "ethers";
import type { Provider } from "@ethersproject/providers";
import type { 
  TypedEventFilter,
  TypedEvent,
  TypedListener,
  OnEvent
} from "./common";

export interface DaoGovernanceInterface extends ethers.utils.Interface {
  functions: {
    "MAX_VOTING_DELAY()": FunctionFragment;
    "MAX_VOTING_PERIOD()": FunctionFragment;
    "cancelProposal(uint256)": FunctionFragment;
    "castVote(uint256,uint8)": FunctionFragment;
    "castVoteWithReason(uint256,uint8,string)": FunctionFragment;
    "executeProposal(uint256)": FunctionFragment;
    "getActions(uint256)": FunctionFragment;
    "getProposalDetails(uint256)": FunctionFragment;
    "getVoteRecord(uint256,address)": FunctionFragment;
    "initialize(address,address[],uint256,uint256)": FunctionFragment;
    "queueProposal(uint256)": FunctionFragment;
    "setVotingDelay(uint256)": FunctionFragment;
    "updateModelGovernance(bytes32,string)": FunctionFragment;
  };

  encodeFunctionData(
    functionFragment: "MAX_VOTING_DELAY",
    values?: undefined
  ): string;
  encodeFunctionData(
    functionFragment: "cancelProposal",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "castVote",
    values: [BigNumber, number]
  ): string;
  encodeFunctionData(
    functionFragment: "castVoteWithReason",
    values: [BigNumber, number, string]
  ): string;
  encodeFunctionData(
    functionFragment: "executeProposal",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "getActions",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "getProposalDetails",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "getVoteRecord",
    values: [BigNumber, string]
  ): string;
  encodeFunctionData(
    functionFragment: "initialize",
    values: [string, string[], BigNumber, BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "queueProposal",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "setVotingDelay",
    values: [BigNumber]
  ): string;
  encodeFunctionData(
    functionFragment: "updateModelGovernance",
    values: [BytesLike, string]
  ): string;

  decodeFunctionResult(
    functionFragment: "MAX_VOTING_DELAY",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "cancelProposal",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "castVote",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "castVoteWithReason",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "executeProposal",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "getActions",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "getProposalDetails",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "getVoteRecord",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "initialize",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "queueProposal",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "setVotingDelay",
    data: BytesLike
  ): Result;
  decodeFunctionResult(
    functionFragment: "updateModelGovernance",
    data: BytesLike
  ): Result;

  events: {
    "ModelGovernanceUpdated(bytes32,string)": EventFragment;
    "ProposalCanceled(uint256)": EventFragment;
    "ProposalCreated(uint256,address,address[],uint256[],string[],bytes[],uint256,uint256,string)": EventFragment;
    "ProposalExecuted(uint256)": EventFragment;
    "ProposalQueued(uint256,uint256)": EventFragment;
    "VoteCast(address,uint256,uint8,uint256,string)": EventFragment;
    "VotingDelaySet(uint256,uint256)": EventFragment;
  };

  getEvent(nameOrSignatureOrTopic: "ModelGovernanceUpdated"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "ProposalCanceled"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "ProposalCreated"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "ProposalExecuted"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "ProposalQueued"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "VoteCast"): EventFragment;
  getEvent(nameOrSignatureOrTopic: "VotingDelaySet"): EventFragment;
}

export interface ModelGovernanceUpdatedEventObject {
  modelHash: string;
  governanceUri: string;
}
export type ModelGovernanceUpdatedEvent = TypedEvent<
  [string, string],
  ModelGovernanceUpdatedEventObject
>;

export interface ProposalCanceledEventObject {
  proposalId: BigNumber;
}
export type ProposalCanceledEvent = TypedEvent<
  [BigNumber],
  ProposalCanceledEventObject
>;

export interface ProposalCreatedEventObject {
  proposalId: BigNumber;
  proposer: string;
  targets: string[];
  values: BigNumber[];
  signatures: string[];
  calldatas: string[];
  startBlock: BigNumber;
  endBlock: BigNumber;
  description: string;
}
export type ProposalCreatedEvent = TypedEvent<
  [
    BigNumber,
    string,
    string[],
    BigNumber[],
    string[],
    string[],
    BigNumber,
    BigNumber,
    string
  ],
  ProposalCreatedEventObject
>;

export interface DaoGovernance extends BaseContract {
  connect(signerOrProvider: Signer | Provider | string): this;
  attach(addressOrName: string): this;
  deployed(): Promise<this>;

  interface: DaoGovernanceInterface;

  queryFilter<TEvent extends TypedEvent>(
    event: TypedEventFilter<TEvent>,
    fromBlockOrBlockhash?: string | number | undefined,
    toBlock?: string | number | undefined
  ): Promise<Array<TEvent>>;

  listeners<TEvent extends TypedEvent>(
    eventFilter?: TypedEventFilter<TEvent>
  ): Array<TypedListener<TEvent>>;
  off<TEvent extends TypedEvent>(
    eventFilter: TypedEventFilter<TEvent>,
    listener: TypedListener<TEvent>
  ): this;
  on<TEvent extends TypedEvent>(
    eventFilter: TypedEventFilter<TEvent>,
    listener: TypedListener<TEvent>
  ): this;
  once<TEvent extends TypedEvent>(
    eventFilter: TypedEventFilter<TEvent>,
    listener: TypedListener<TEvent>
  ): this;
  removeListener<TEvent extends TypedEvent>(
    eventFilter: TypedEventFilter<TEvent>,
    listener: TypedListener<TEvent>
  ): this;
  removeAllListeners<TEvent extends TypedEvent>(
    eventFilter: TypedEventFilter<TEvent>
  ): this;

  listeners(eventName?: string): Array<Listener>;
  off(eventName: string, listener: Listener): this;
  on(eventName: string, listener: Listener): this;
  once(eventName: string, listener: Listener): this;
  removeListener(eventName: string, listener: Listener): this;
  removeAllListeners(eventName?: string): this;

  populateTransaction: {
    MAX_VOTING_DELAY(): Promise<PopulatedTransaction>;
    cancelProposal(proposalId: BigNumber): Promise<PopulatedTransaction>;
    castVote(
      proposalId: BigNumber,
      support: number
    ): Promise<PopulatedTransaction>;
    castVoteWithReason(
      proposalId: BigNumber,
      support: number,
      reason: string
    ): Promise<PopulatedTransaction>;
    executeProposal(proposalId: BigNumber): Promise<PopulatedTransaction>;
    getActions(proposalId: BigNumber): Promise<PopulatedTransaction>;
    getProposalDetails(
      proposalId: BigNumber
    ): Promise<PopulatedTransaction>;
    getVoteRecord(
      proposalId: BigNumber,
      voter: string
    ): Promise<PopulatedTransaction>;
    initialize(
      admin: string,
      executors: string[],
      votingDelay: BigNumber,
      votingPeriod: BigNumber
    ): Promise<PopulatedTransaction>;
    queueProposal(proposalId: BigNumber): Promise<PopulatedTransaction>;
    setVotingDelay(newVotingDelay: BigNumber): Promise<PopulatedTransaction>;
    updateModelGovernance(
      modelHash: BytesLike,
      governanceUri: string
    ): Promise<PopulatedTransaction>;
  };
}

export type ProposalState = 
  | "Pending"
  | "Active"
  | "Canceled"
  | "Defeated"
  | "Succeeded"
  | "Queued"
  | "Expired"
  | "Executed";

export type VoteType = 
  | "Against"
  | "For"
  | "Abstain";

export interface ProposalDetails {
  proposer: string;
  targets: string[];
  values: BigNumber[];
  signatures: string[];
  calldatas: string[];
  startBlock: BigNumber;
  endBlock: BigNumber;
  descriptionHash: string;
  state: ProposalState;
}

export interface VoteRecord {
  votes: BigNumber;
  support: VoteType;
  reason: string;
}
