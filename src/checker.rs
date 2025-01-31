use crate::{
    output::print_result,
    query::{add_result_to_channel, QueryError, QueryResult, QueryStatus},
    requests::RequestResult,
    sherlock_target_manifest::{ErrorType, TargetInfo},
    waf::waf_hit,
};
use color_eyre::eyre;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::channel;

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub timeout: u64,
    pub proxy: Arc<Option<String>>,
    pub print_all: bool,
    pub print_found: bool,
    pub dump_response: bool,
    pub browse: bool,
}

pub async fn check_username(
    username: &str,
    site_data: Arc<HashMap<String, Arc<TargetInfo>>>,
    options: &CheckOptions,
) -> color_eyre::Result<Vec<QueryResult>> {
    let CheckOptions {
        timeout,
        proxy,
        print_all,
        print_found,
        dump_response,
        browse,
    } = options;

    let num_of_sites = site_data.keys().len();
    if num_of_sites == 0 {
        return Err(eyre::eyre!("No sites to check"));
    }

    let (tx, mut rx) = channel::<RequestResult>(num_of_sites);

    // ping sites for username matches
    let username = Arc::new(username.to_string());
    for (site, info) in site_data.iter() {
        add_result_to_channel(
            tx.clone(),
            Arc::clone(&username),
            Arc::new(site.to_string()),
            Arc::clone(&info),
            *timeout,
            Arc::clone(proxy),
        )?;
    }

    drop(tx);

    // save to output data struct
    let mut results = vec![];
    while let Some(result) = rx.recv().await {
        let site = result.site;
        let info = result.info;
        let url = result.url;
        let url_probe = result.url_probe;
        let username = result.username;

        let query_result: QueryResult = match result.response {
            Err(e) => match e {
                QueryError::InvalidUsernameError => QueryResult {
                    username: Arc::clone(&username),
                    site_name: Arc::clone(&site),
                    info: Arc::clone(&info),
                    site_url_user: url,
                    status: QueryStatus::Illegal,
                    http_status: None,
                    query_time: result.query_time,
                    context: Some(e.to_string()),
                },
                QueryError::RequestError => QueryResult {
                    username: Arc::clone(&username),
                    site_name: Arc::clone(&site),
                    info: Arc::clone(&info),
                    site_url_user: url,
                    status: QueryStatus::Unknown,
                    http_status: None,
                    query_time: result.query_time,
                    context: Some(e.to_string()),
                },
            },
            Ok(response) => {
                let status_code = response.status().as_u16();
                let resp_text = response.text().await?;
                let wfthit = waf_hit(&resp_text);

                let error_type = &info.error_type;
                let error_code = &info.error_code;
                let status = match (wfthit, &error_type) {
                    (true, _) => QueryStatus::Waf,
                    (false, ErrorType::Message) => {
                        let error_flag = info.error_msg.iter().any(|msg| msg.is_in(&resp_text));
                        if error_flag {
                            QueryStatus::Available
                        } else {
                            QueryStatus::Claimed
                        }
                    }
                    (false, ErrorType::StatusCode) => {
                        let mut status = QueryStatus::Claimed;

                        if let Some(error_codes) = &error_code {
                            if error_codes.contains(&status_code) {
                                status = QueryStatus::Available;
                            }
                        } else if !(200..=399).contains(&status_code) {
                            status = QueryStatus::Available;
                        }

                        status
                    }
                    (false, ErrorType::ResponseUrl) => {
                        if (200..300).contains(&status_code) {
                            QueryStatus::Claimed
                        } else {
                            QueryStatus::Available
                        }
                    }
                };

                if *dump_response {
                    println!("+++++++++++++++++++++");
                    println!("TARGET NAME   : {site}");
                    println!("USERNAME      : {username}");
                    println!("TARGET URL    : {:?}", url_probe);
                    println!("TEST METHOD   : {:?}", error_type);
                    if let Some(error_codes) = &error_code {
                        println!("ERROR CODES   : {:?}", error_codes);
                    }
                    println!("Results...");
                    println!("RESPONSE CODE : {}", status_code);
                    if let Some(error_msg) = &info.error_msg {
                        println!("ERROR TEXT    : {:?}", error_msg);
                    }
                    println!(">>>>> BEGIN RESPONSE TEXT");
                    println!("{}", resp_text);
                    println!("<<<<< END RESPONSE TEXT");

                    println!("VERDICT       : {:?}", status);
                    println!("+++++++++++++++++++++");
                }

                if *browse && status == QueryStatus::Claimed {
                    open::that(&url).inspect_err(|e| eprintln!("Failed to open browser: {}", e))?;
                }

                QueryResult {
                    username: Arc::clone(&username),
                    site_name: Arc::clone(&site),
                    info: Arc::clone(&info),
                    site_url_user: url,
                    status,
                    http_status: Some(status_code),
                    query_time: result.query_time,
                    context: None,
                }
            }
        };

        if *print_all || (*print_found && query_result.status == QueryStatus::Claimed) {
            print_result(&query_result);
        }
        results.push(query_result);
    }

    Ok(results)
}
