# Prediction Market IOTA Smart Contract for Demonstration
 
---
Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.


# Setup

### Environment
- Ubuntu 20.04 LTS with latest updates and upgrades
- installed __git__
- installed [__go__ 1.16.5](https://golang.org/doc/install)
- installed __rust__ 1.53.0 for developing native IOTA smart contracts by
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- installed __wasm-pack__ 0.10.0 for compiling smart contracts into "WebAssembly" binaries by
  `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`
---
- installed [dependencies for rocksdb](https://github.com/facebook/rocksdb/blob/master/INSTALL.md) required by Goshimmer
* `sudo apt-get install libgflags-dev`
* `sudo apt-get install libsnappy-dev`
* `sudo apt-get install zlib1g-dev`
* `sudo apt-get install libbz2-dev`
* `sudo apt-get install liblz4-dev`
* `sudo apt-get install libzstd-dev`
---
- downloaded and built Goshimmer **0.7.5** from the **develop** branch (in the version of 2021-08-16) with the latest commit before the clone being [this](https://github.com/iotaledger/goshimmer/commit/c507a3ae28bc9d92f722334a44ebc20444d688ca).
* `git clone -b develop https://github.com/iotaledger/goshimmer.git`
* checked out a workable state of the branch like the commit suggested above
* `cd goshimmer`
* `go install`
* `go build -tags rocksdb`
---
- prepared goshimmer for transaction handling
  Save the [_config.json_ file](https://github.com/51nodes/prediction-market-smart-contract/blob/main/goshimmer/config.json) to your goshimmer directory to
* enable the **txstream** plugin, which allows goshimmer to communicate with wasp nodes
* disable the **portcheck** plugin, which tries to connect to the remote Devnet

Run goshimmer in its directory as follows:

`./goshimmer  --autopeering.seed=base58:8q491c3YWjbPwLmF2WD95YmCgh61j2kenCKHfGfByoWi  --node.enablePlugins=bootstrap,prometheus,spammer,"webapi tools endpoint",activity,snapshot,txstream   --messageLayer.startSynced=true   --autopeering.entryNodes=       --node.disablePlugins=clock       --messageLayer.snapshot.file=./assets/snapshotTest.bin       --messageLayer.snapshot.genesisNode=       --metrics.manaResearch=false       --mana.enableResearchVectors=false       --mana.snapshotResetTime=true       --statement.writeStatement=true --statement.writeManaThreshold=1.0 --config=./config.json`


### Create a cli wallet
Install the cli-wallet in a new directory

* `wget https://github.com/iotaledger/goshimmer/releases/tag/v0.7.5 download cli-wallet-0.7.5_Linux_x86_64.tar.gz`
* `tar -xf cli-wallet-0.7.5_Linux_x86_64.tar.gz`

Set reuse_addresses=true in the config.json of cli-wallet:
```json
{
  "WebAPI": "http://127.0.0.1:8080",
  "basic_auth": {
    "enabled": false,
    "username": "goshimmer",
    "password": "goshimmer"
  },
  "reuse_addresses": true,
  "faucetPowDifficulty": 25,
  "assetRegistryNetwork": "nectar"
}
```

To create a new wallet run
`./cli-wallet init`

Note _your_ SEED for allocating funds to this wallet.

We generate a custom genesis snapshot, with the transaction that allocates the funds.

Go to the goshimmer installation directory and then to the following subdirectory ./tools/genesis-snapshot

Paste the seed of the previously generated cli wallet to the following command

`go run main.go --token-amount 3500000 --seed E7owJWtDBGSUAZUWQkn1kHG5zUy2PLQf6eEr3RoMCJs7 --snapshot-file snapshotTest.bin`

Now,
* go to the goshimmerdev directory and inside of it run
* `mkdir assets`
* `cp ./tools/genesis-snapshot/snapshotTest.bin ./assets/snapshotTest.bin`
  to provide the generated snapshotTest.bin file to goshimmer.


### Setting up a Wasp node for smart contracts

We installed Wasp from the **master** branch in the state of 2021-08-03.
* `git clone https://github.com/iotaledger/wasp.git`
* checkout a workable state of the code from the repository. We used this [commit](https://github.com/iotaledger/wasp/commit/05516ca29edd9e93b17ed9a0f788ddb51c407d48).
* `cd wasp`
* `go install`
* `go build -tags rocksdb`
* `go build -tags rocksdb ./tools/wasp-cli`
* `./wasp`

We need to transfer funds to the wasp wallet by creating the wallet in the first place by
`/wasp-cli init`

We need to get the address of the wallet by
`./wasp-cli balance`

To send funds to this wallet, paste _your_ address into this command and run it in the cli-wallet's directory:

`./cli-wallet send-funds -amount 40000 -dest-addr 1Ah4cqMPdrDGx6Htapk7NZUxxcYHsP1C3oAugEYHVmACj`

Finally, configure wasp-cli to be able to connect to the local goshimmer node and to form a committee of
one local Wasp node by saving the [_wasp-cli.json_ file](https://github.com/51nodes/prediction-market-smart-contract/blob/main/wasp/wasp-cli.json) to the directory of wasp-cli.


### Deploying a chain
Smart contracts are deployed on a chain, which needs to be deployed first:

`./wasp-cli chain deploy --committee=0 --quorum=1 --chain=predmarketchain --description="Prediction Market"`

Now we have to provide funds to the chain by

`./wasp-cli chain deposit IOTA:1000 --chain=predmarketchain`


# A Prediction Market Smart Contract

### Design

Our design of a prediction market for demonstration purposes is simple.
We omit a book maker and a pricing mechanism.
Formally, we do not pose a question with pre-defined possible outcomes.
Instead, market participants can bet basically on any outcome of an event with an arbitrary amount of tokens until the time for predictions is over.
Afterwards, the winning outcome is determined and winning bets placed on the correct outcome receive back their share on the overall amount of tokens placed in bets.
Assume, in total 700 tokens were bet on "no" and 300 tokens in total were bet on "yes", and "yes" is the actual outcome.
A single bet on "yes" with 100 tokens receives (100/300)*(700+300) = 333 tokens, making a win of 233 tokens.

Realizing this design as a smart contract in the IOTA network allows to deploy one contract per question to be answered.
The account deploying the contract is in control and has to specify the time until when bets can be placed on outcomes.
The actual question to be answered has to be communicated in third party channels.
Any network participant can then look up the contract and call a function to place a bet on an outcome by sending some IOTA from their wallet.
Finally, after prediction time has passed, the account deployer has to call a function to close the prediction market and to provide the actual outcome of the event and correct answer to the question.
This triggers the evaluation of all bets with regard to the correct answer.
Accounts with the correct answer receive the winning amount of IOTAs in a transaction.

### Build
To build the smart contract, pull the [repository](https://github.com/51nodes/prediction-market-smart-contract)

To define dependencies of the smart contract code, the [_Cargo.toml_ file](https://github.com/51nodes/prediction-market-smart-contract/blob/main/Cargo.toml) is used.

To build the smart contract, run `wasm-pack build` in the directory where _Cargo.toml_ resides.
The compiled WebAssembly file is located in the pkg directory and named _predictionmarket_bg.wasm_.


### Execution and Testing

Now we deploy our simple smart contract compiled as a WebAssembly _wasm_ file.

Note: please adapt the path to the wasm file if required.

`./wasp-cli chain deploy-contract wasmtime predictionmarket "Prediction Market SC" ./prediction-market-smart-contract/pkg/predictionmarket_bg.wasm  --chain=predmarketchain --upload-quorum=1 -d --address-index=0`

Now, functions can be called on the contract. First, the same wasp-cli that deployed the contract needs to call the initmarket function.
There are two posibilities:

a) do not specify a specific end date for the prediction market to simplify testing and development
`./wasp-cli chain post-request predictionmarket initmarket --chain=predmarketchain`

b) specify a specific end date and time for the prediction market. The iso format is used and UTC is assumed.
In this way, all bets must be placed before this time and the market can be only closed after this time.
`./wasp-cli chain post-request predictionmarket initmarket string BETENDUTC string "2021-09-08 23:00" --chain=predmarketchain`

For the deployed prediction market, we assume two possible outcomes "yes" and "no" on which bets can be submitted.
To place a bet with 10 IOTA on "no", we run

`./wasp-cli chain post-request predictionmarket bet string BETVALUE string no --chain=predmarketchain -t IOTA:10`

The contract owner (with the first wallet) can close the prediction market by running in the directory of the first wallet

`./wasp-cli chain post-request predictionmarket closemarket string BETVALUE string no --chain=predmarketchain`

In this example, the actual outcome is specified to be "no".

### Limitations

There are some limitations of the presented prediction market

* Only one contract per chain can be deployed because the bets are not stored per contract identification in the chain's state
* All bets are stored on-chain, so they are public
* Each account (given by a wasp wallet) can place only one bet per deployed prediction market contract
* The actual question asked by the prediction market and the possible outcomes have to be conveyed informally
* Bets are against other market participants - there is no market maker