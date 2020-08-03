use insta::assert_debug_snapshot;
use std::collections::BTreeMap;
use colored::Colorize;

use anyhow::{
    anyhow,
    // Context as _,
};

pub use super::{
    parse_response,
    Response,
};

fn responses_for(v: &str) -> anyhow::Result<Vec<Response>> {
    std::iter::repeat(true)
        .scan(v, |acc, _| {
            let mut try_parse = move || {
                if acc.len() == 0 {
                    return Ok(None)
                }

                let (
                    remaining,
                    (parsed, response)
                ) = parse_response(acc)
                    .map_err(|err| {
                        anyhow!(
                            "[{}]\n Error: {:?}. Input: {:?}\n",
                            "ERROR".red(),
                            err,
                            acc,
                        )
                    })?;

                if let Response::Unknown = response {
                    Err(anyhow!(
                        "[{}]\nParsed: {:?} . Input: {:?}\n",
                        "UNKNOWN RESPONSE".red(),
                        parsed,
                        acc,
                    ))?;
                };
    
                *acc = remaining;

                Ok(Some(response))
            };

            try_parse().transpose()
        })
        .collect::<anyhow::Result<Vec<Response>>>()
}

#[test]
fn test_parse_response() -> anyhow::Result<()> {
    let data = include_str!("data/ender3_marlin_2019_firmware.toml");
    let data: BTreeMap<String, BTreeMap<String, String>> = toml::from_str(data)?;

    // let mut data: Vec<(String, String)> = data
    //     .iter()
    //     .map(|(k, v)| {
    //         let k = k.clone();
    //         v.clone().into_iter().map(move |(k2, v)| {
    //             (format!("{}::{}", &k, k2), v)
    //         })
    //     })
    //     .flatten()
    //     .collect();

    // data.sort_by_key(|(k, _)| k.to_owned());

    type SectionResponseTree = BTreeMap<String, Vec<Response>>;
    type Snapshot = BTreeMap<String, SectionResponseTree>;

    let responses: anyhow::Result<Snapshot> = data
        .iter()
        .map(|(section_title, scenarios)| {
            let section_title = section_title.clone();

            let section_responses = scenarios.clone()
                .iter()
                .map(|(k, v)| {
                    let title = format!("{}::{}", &section_title, k);

                    print!("\nTesting {:?} ", title);
                    let res = responses_for(&v)?;

                    println!("[{}]", "DONE".green());
                    println!("Got: {:?}\n", res);

                    Ok((k.to_owned(), res))
                })
                .collect::<anyhow::Result<SectionResponseTree>>()?;
            
            Ok((section_title, section_responses))
        })
        .collect();

    assert_debug_snapshot!(responses?);

    Ok(())
}
