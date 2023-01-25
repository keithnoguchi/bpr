import * as anchor from "@project-serum/anchor";
import idl from "../../../target/idl/anchor_multisig.json";
import programKeypair from "../../../target/deploy/anchor_multisig-keypair.json"

import { SystemProgram } = anchor.web3;

export default class AnchorClient {
  constructor({ programId, config, keypair } = {}) {
    this.programId = programId;
    this.config = config;
    this.connection = new anchor.web3.Connection(this.config.httpUri, 'confirm');
    console.log('\n\nConnected to', this.config.httpUri);

    const wallet =
      window.solana.isConnected && window.solana?.isPhantom
        ? new WalletAdaptorPhantom()
        : keypair
        ? new anchor.Wallet(keypair)
        : new anchor.Wallet(anchor.web3.Keypair.generate());

    this.provider = new anchor.Provider(this.connection, wallet, opts);
    this.program = new anchor.Program(idl, this.programId, this.provider);
  }
}
