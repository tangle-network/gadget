import * as fs from "node:fs/promises";
import * as path from "node:path";
import { cryptoWaitReady } from "@polkadot/util-crypto";
import {
  http,
  defineChain,
  createWalletClient,
  createPublicClient,
  getContractAddress,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";
import * as blueprint from "./blueprint.json";
import { AddressOrPair, SubmittableExtrinsic } from "@polkadot/api/types";

const tangle = defineChain({
  id: 3287,
  name: "Tangle Testnet",
  nativeCurrency: {
    decimals: 18,
    name: "Tangle Test Network Token",
    symbol: "TTNT",
  },
  rpcUrls: {
    default: {
      http: ["http://localhost:9944"],
      webSocket: ["ws://localhost:9944"],
    },
    remote: {
      http: ["https://testnet-rpc.tangle.tools"],
      webSocket: ["wss://testnet-rpc.tangle.tools"],
    },
  },
  blockExplorers: {
    default: { name: "Explorer", url: "http://localhost:3000" },
    remote: { name: "Explorer", url: "https://testnet-explorer.tangle.tools" },
  },
});

await cryptoWaitReady();
const sr25519 = new Keyring({ type: "sr25519" });
const ecdsa = new Keyring({ type: "ecdsa" });
const aliceSr25519 = sr25519.addFromUri("//Alice");
const aliceEcdsa = ecdsa.addFromUri("//Alice");
const bobSr25519 = sr25519.addFromUri("//Bob");
const DEPLOYER = privateKeyToAccount(
  "0x99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342",
);

const walletClient = createWalletClient({
  chain: tangle,
  transport: http(),
});

const publicClient = createPublicClient({
  chain: tangle,
  transport: http(),
});

const deployContract = async (path: string) => {
  const json = await fs.readFile(path, "utf-8");
  const contractJson = JSON.parse(json);
  const contractAddress = getContractAddress({
    from: DEPLOYER.address,
    nonce: await publicClient
      .getTransactionCount({
        address: DEPLOYER.address,
      })
      .then((n) => BigInt(n)),
    opcode: "CREATE",
  });
  await walletClient.deployContract({
    abi: contractJson.abi as [],
    bytecode: contractJson.bytecode.object,
    account: DEPLOYER,
  });
  return contractAddress;
};

console.log("|- Using account to deploy contracts...", DEPLOYER.address);
console.log("|-- Deploying contracts...");
for (const job of blueprint.jobs) {
  // get the absolute path of the contract
  const realPath = path.resolve(job.verifier.Evm);
  const verifier = await deployContract(realPath);
  job.verifier.Evm = verifier;
  console.log(
    `|--- Verifier contract for ${job.metadata.name} deployed at: ${verifier}`,
  );
}

console.log();

const api = await ApiPromise.create({
  provider: new WsProvider("ws://localhost:9944"),
  noInitWarn: true,
});

await api.isReady;
const blueprintId = await api.query.services.nextBlueprintId();

const createBlueprintTx = api.tx.services.createBlueprint(blueprint);
console.log(`|- Creating blueprint with id: ${blueprintId}`);
await signAndSend(aliceSr25519, createBlueprintTx);
console.log();

console.log("|- Registering Alice as an Operator on the blueprint...");

const registerTx = api.tx.services.register(
  blueprintId,
  {
    key: aliceEcdsa.publicKey,
    approval: "None",
  },
  [],
);
await signAndSend(aliceSr25519, registerTx);
console.log();

const serviceInstanceId = await api.query.services.nextInstanceId();
console.log(
  `|- Bob is creating service instance with id: ${serviceInstanceId}`,
);
const requestServiceTx = api.tx.services.request(
  blueprintId,
  [],
  [aliceSr25519.address],
  10_000,
  [],
);
await signAndSend(bobSr25519, requestServiceTx);
console.log();

const jobCallId = await api.query.services.nextJobCallId();
console.log(`|- Bob is calling job with id: ${jobCallId}`);
const x = 5;
const xsquareJobTx = api.tx.services.call(serviceInstanceId, 0, [
  { Uint64: x },
]);
await signAndSend(bobSr25519, xsquareJobTx);
console.log();
console.log("Done!");
await api.disconnect();

// *** --- Helper Functions --- ***

async function signAndSend(
  signer: AddressOrPair,
  tx: SubmittableExtrinsic<"promise">,
  waitTillFinallized: boolean = false,
): Promise<void> {
  // Sign and send the transaction and wait for it to be included.
  await new Promise(async (resolve, reject) => {
    const unsub = await tx.signAndSend(
      signer,
      async ({ events = [], status, dispatchError }) => {
        if (dispatchError) {
          if (dispatchError.isModule) {
            // for module errors, we have the section indexed, lookup
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            const { docs, name, section } = decoded;

            console.log(`|--- ${section}.${name}: ${docs.join(" ")}`);
            reject(`${section}.${name}`);
          }
        }
        if (status.isInBlock && !waitTillFinallized) {
          console.log("|--- Events:");
          events.forEach(({ event: { data, method, section } }) => {
            console.log(`|---- ${section}.${method}:: ${data}`);
          });
          unsub();
          resolve(void 0);
        }
        if (status.isFinalized && waitTillFinallized) {
          console.log("|--- Events:");
          events.forEach(({ event: { data, method, section } }) => {
            console.log(`|---- ${section}.${method}:: ${data}`);
          });
          unsub();
          resolve(void 0);
        }
      },
    );
  });
}
