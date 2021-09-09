// Demo IOTA smart contract with a simple prediction market
//
// Bets can be placed on arbitrary outcome values (e.g., "yes" or "no") of arbitrary events with arbitrary bet sizes by sending IOTA with the transaction.
// Bets can be placed until a specified time when the prediction market ends, specified by the contract owner.
// When the contract owner closes the market, the winning value has to be provided, e.g. "yes".
// The winning bets automatically receive IOTA proportional to their bet size.
//   Assume 700 IOTA were bet on "no" and 300 IOTA on "yes", and "yes" is the actual outcome.
//   A bet on "yes" with 100 IOTA receives (100/300)*1000 = 333 IOTA
//
// Assumes only one contract per chain. To allow multiple contracts, bets need to be stored in a map per id of the contract.
// Note that bets are stored in the contract's state, so in principle, they can be publicly accessed, although the contract itself does not provide a function to do so.
// Note that when sending IOTA to the betters, a minimum transaction fee of 1 IOTA is deducted.
//
// author: achim.klein@51nodes.io
// date: 2021-09-07
// version: 1.0
// license: Apache License 2.0


use wasmlib::*;
use chrono::{DateTime,  Utc, NaiveDateTime};
use serde_with::serde_as;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;



#[no_mangle]
fn on_load() {
    // functions of the smart contract
    let exports = ScExports::new();
    exports.add_func("bet", bet );
    exports.add_func("initmarket", initmarket);
    exports.add_func("closemarket", closemarket);
}

// The contract owner should call this function for initialization and to set an end time for betting 
// using the parameter BETENDUTC, which is a date and time string in ISO format, assuming UTC.
fn initmarket(context: &ScFuncContext) {
    // only contract owner should be able to do this
    let creator = context.contract_creator();
    let caller = context.caller();
    context.require(creator == caller, "Not authorised to init market - only contract creator is allowed to do this.");

    let mut log:String = "INITMARKET is run:".to_string();   context.log(&log);
    
    // a flag, stating that the closemarket function was not (successfully) run yet
    context.state().get_string("marketclosed").set_value(&"false".to_string());

    if context.params().get_string(&"BETENDUTC".to_string()).value()==""  {
        // default: do not use end time for bets
        context.state().get_int64(&"betenddatetime".to_string()).set_value(0);

        log = "Do not use specific end time for bets".to_string();  context.log(&log);
    }
    else {
        // parse ISO datetime string, e.g. "2021-01-01 02:00" (in UTC) and convert to UNIX timestamp
        let betenddatetime:i64 = DateTime::<Utc>::from_utc(NaiveDateTime::parse_from_str(&context.params().get_string(&"BETENDUTC".to_string()).value(), "%Y-%m-%d %H:%M").expect("failed to execute"), Utc).timestamp();

        log = "Bet end timestamp (UTC): ".to_string() + &betenddatetime.to_string();     context.log(&log);

        // store state
        context.state().get_int64(&"betenddatetime".to_string()).set_value(betenddatetime);
    }
}

#[serde_as]
#[derive(Deserialize, Serialize)]
struct Bet {
    // bet size in IOTA
    betamount: i32,
    // value for which the bet is valid, e.g., "yes" or "no" regarding a question or an outcome of an event
    betisforvalue: String,
}

#[serde_as]
#[derive(Deserialize, Serialize)]
struct ContainerOfBets {
    // map betting account's wallet address (string) to a Bet
    map: HashMap<String,Bet>,
}


// function to place a bet on a certain value provided as parameter BETVALUE, e.g. "yes"
// the amount to bet is the amount of IOTA sent with the function call
// bets must be placed in time before the betenddatetime has passed set on initialization
fn bet(context: &ScFuncContext) {
    let currtime:i64 = context.timestamp();  // transaction timestamp?!
    let betenddatetime:i64 = context.state().get_int64(&"betenddatetime".to_string()).value();

    // either we don't use a fixed end time - or we check if the end time is not exceeded
    if betenddatetime==0 || (betenddatetime!=0 && currtime <= betenddatetime) {
        let mut log:String = "BET is placed:".to_string(); context.log(&log);

        // how much IOTA were sent with the transaction?
        let incoming = context.incoming().balance(&ScColor::IOTA);
        log = "bet amount (IOTA): ".to_string() + &incoming.to_string();   context.log(&log);
      
        // get outcome value on which the bet was placed
        let betvalue = context.params().get_string(&"BETVALUE".to_string());
        // require parameter exists
        context.require(betvalue.exists(), "bet value parameter not found");

        // get wallet address of betting account
        let caller = context.caller().address();
        // store the value the bet refers to, e.g., "yes" or "no" - per betting account
        context.state().get_map(&caller.to_string()).get_string(&"betvalue".to_string()).set_value(&betvalue.to_string());
        
        // store all bets as jsonified hashmap in the state, which does not allow iterating over a map
        let containerofbetsjson = context.state().get_string(&"containerofbetsjson".to_string()).value();
        let mut containerofbets : ContainerOfBets;

        // already stored?
        if containerofbetsjson == "" {
            containerofbets = ContainerOfBets {
                map : HashMap::new()
            };
        }
        else {
            // de-serialize and re-create the struct from string
            containerofbets = serde_json::from_str(&containerofbetsjson).expect("failed to get container of bets");
        }

        // create Bet struct and store in map under the betting account's (wallet) address
        let bet = Bet  {
            betamount: incoming.to_string().parse::<i32>().unwrap(),
            betisforvalue: betvalue.to_string(),
        };
        containerofbets.map.insert(caller.to_string(), bet);

        // serialize all bets to a json string
        let containerofbetsjson = serde_json::to_string(&containerofbets).expect("failed to make json of container of bets");
        // store state as a string
        context.state().get_string(&"containerofbetsjson".to_string()).set_value(&containerofbetsjson);
    } else {
        let log:String = "bet was not provided on time".to_string();
        context.log(&log);
    }
}


// Function to close the prediction market, to be called by the contract owner.
// The function requires a BETVALUE parameter, specifying the winning outcome, e.g., "yes".
// The functions runs through the stored bets, determines winning bets and the amount of IOTA the receive, and sends the IOTA to the wallets of the winners.
fn closemarket(context: &ScFuncContext) {
    // only contract owner should be able to do this
    let creator = context.contract_creator();
    let caller = context.caller();
    context.require(creator == caller, "You are not authorised to close the prediction market - only contract creator is allowed to close the market.");

    // the value that won, e.g., "yes" or "no"
    let betvaluewinning = context.params().get_string(&"BETVALUE".to_string());
    // require parameter exists
    context.require(betvaluewinning.exists(), "winning bet value parameter not found");

    // only close market after end time for bets, specified on initalization
    let currtime: i64 = context.timestamp();
    let betenddatetime: i64 = context.state().get_int64(&"betenddatetime".to_string()).value();

    let mut log:String;

    // a flag to check whether the closemarket function was run
    let marketclosed: String = context.state().get_string("marketclosed").to_string();
    if marketclosed.eq(&"false".to_string()) {
        // either we don't use a fixed end time - or we check if the end time is exceeded
        if betenddatetime == 0 || (betenddatetime != 0 && currtime > betenddatetime) {
            log = "CLOSEMARKET is executed:".to_string(); context.log(&log);
            log = "the winning value is: \"".to_string() + &betvaluewinning.to_string() + &"\"".to_string(); context.log(&log);

            // set flag stating that the closemarket function was run
            context.state().get_string("marketclosed").set_value(&"true".to_string());

            // get all bets from global state
            // Note that the stat is not specific to a contract but to the whole chain on which it is deployed.
            let containerofbetsjson = context.state().get_string(&"containerofbetsjson".to_string()).value();
            let containerofbets: ContainerOfBets;

            if containerofbetsjson != "" {
                // get bets from json
                containerofbets = serde_json::from_str(&containerofbetsjson).expect("failed to fetch container of bets");
                // we require more than one bet
                if containerofbets.map.keys().len() >= 1 {
                    // determine total amount of bet amounts per value, e.g., 500 IOTA on "yes" and 2000 IOTA on "no"
                    let mut betvalue_totalbetamount: HashMap<String, i32> = HashMap::new();
                    // overall amount in bets, regardless on which outcome value the bet was placed
                    let mut totalbetamount:i32 = 0;
                    for (_betteraddress, bet) in &containerofbets.map {
                        totalbetamount = totalbetamount + bet.betamount;
                        if betvalue_totalbetamount.contains_key(&bet.betisforvalue) {
                            betvalue_totalbetamount.insert((&bet.betisforvalue).parse().unwrap(), betvalue_totalbetamount.get(&bet.betisforvalue).unwrap() + bet.betamount);
                        } else {
                            betvalue_totalbetamount.insert((&bet.betisforvalue).parse().unwrap(), bet.betamount);
                        }
                    }

                    // log output
                    for (betvalue, totalbetamount) in & betvalue_totalbetamount{
                        log = "total amount of bets placed on \"".to_string() + &betvalue.to_string() + &"\" is ".to_string() + &totalbetamount.to_string() + &" IOTA".to_string(); context.log(&log);
                    }
                    log = "total amount of bets over all values: ".to_string() + &totalbetamount.to_string() + &" IOTA".to_string(); context.log(&log);

                    let mut totalbetamountforvalue: Option<&i32>;
                    let mut winamount:i64;
                    let mut recipientaddress:ScAddress;
                    // send coins to winners
                    for (betteraddress, bet) in &containerofbets.map {
                        if bet.betisforvalue.eq(&betvaluewinning.to_string()) {
                            log = betteraddress.to_string() + &" placed a bet on \"".to_string() + &bet.betisforvalue.to_string() + &"\", which is a WIN".to_string(); context.log(&log);
                            totalbetamountforvalue  = betvalue_totalbetamount.get(&bet.betisforvalue);
                            winamount = ((bet.betamount as f32/ *totalbetamountforvalue.unwrap() as f32) * totalbetamount as f32) as i64;
                            log = "bet amount: ".to_string() + &bet.betamount.to_string() + &" IOTA; won amount: " + &winamount.to_string() + &" IOTA; of total amount placed a bet on " + &totalbetamount.to_string() + &"; where total amount per winning value: " + &totalbetamountforvalue.unwrap().to_string();    context.log(&log);
                            if winamount>0 {
                                recipientaddress = ScAddress::from_bytes(&*context.utility().base58_decode(&betteraddress.to_string()));
                                log = "transferring won amount of IOTA to: ".to_string() +  &recipientaddress.to_string();  context.log(&log);
                                context.transfer_to_address( &recipientaddress, ScTransfers::new(&ScColor::IOTA, winamount))
                            }
                        }
                        else  {
                            log = betteraddress.to_string() + &" placed a bet on \"".to_string() + &bet.betisforvalue.to_string() + &"\", which is not a win".to_string(); context.log(&log);
                        }
                    }
                } else {
                    log  = "at least one bet is required".to_string(); context.log(&log);
                }
            } else {
                log  = "no bets stored".to_string(); context.log(&log);
            }
        } else {
            log  = "closing the market can be only done after the end time for placing bets has passed".to_string(); context.log(&log);
        }    
    } else {
        log  = "the prediction market was already closed".to_string(); context.log(&log);
    }
    
}
