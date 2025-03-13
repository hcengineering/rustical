use crate::auth::HulyUser;
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION};
use rustical_store::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) type Timestamp = i64;

pub(crate) const CLASS_EVENT: &str = "calendar:class:Event";
pub(crate) const CLASS_RECURRING_EVENT: &str = "calendar:class:ReccuringEvent";
pub(crate) const CLASS_RECURRING_INSTANCE: &str = "calendar:class:ReccuringInstance";
pub(crate) const CLASS_TX_CREATE_DOC: &str = "core:class:TxCreateDoc";
pub(crate) const CLASS_TX_REMOVE_DOC: &str = "core:class:TxRemoveDoc";
pub(crate) const CLASS_TX_UPDATE_DOC: &str = "core:class:TxUpdateDoc";
pub(crate) const SPACE_CALENDAR: &str = "calendar:space:Calendar";
pub(crate) const SPACE_TX: &str = "core:space:Tx";
pub(crate) const ID_NOT_ATTACHED: &str = "calendar:ids:NoAttached";
pub(crate) const COLLECTION_EVENTS: &str = "events";

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyCalendar {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    pub(crate) modified_on: Timestamp,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventSlim {
    pub(crate) event_id: String,
    pub(crate) modified_on: Timestamp,
    pub(crate) recurring_event_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventData {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    #[serde(rename = "_class")]
    pub(crate) class: String,
    pub(crate) space: String,
    pub(crate) collection: String,
    pub(crate) attached_to: String,
    pub(crate) attached_to_class: String,
    pub(crate) event_id: Option<String>,
    pub(crate) modified_by: String,
    pub(crate) modified_on: Timestamp,
    pub(crate) created_on: Timestamp,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) location: Option<String>,
    pub(crate) all_day: bool,
    pub(crate) date: Timestamp,
    pub(crate) due_date: Timestamp,
    pub(crate) participants: Vec<String>,
    pub(crate) external_participants: Option<Vec<String>>,
    pub(crate) reminders: Option<Vec<Timestamp>>,
    pub(crate) time_zone: Option<String>,
    pub(crate) rules: Option<Vec<RecurringRule>>,
    pub(crate) exdate: Option<Vec<Timestamp>>,
    pub(crate) is_cancelled: Option<bool>,
    pub(crate) original_start_time: Option<Timestamp>,
}

impl HulyEventData {
    #[allow(dead_code)]
    pub(crate) fn pretty_str(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEvent {
    pub(crate) data: HulyEventData,
    pub(crate) instances: Option<Vec<HulyEventData>>,
}

impl HulyEvent {
    #[allow(dead_code)]
    pub(crate) fn pretty_str(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecurringRule {
    pub(crate) freq: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) end_date: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_second: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_minute: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_hour: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_day: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_month_day: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_year_day: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_week_no: Option<Vec<i8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_month: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) by_set_pos: Option<Vec<i16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) wkst: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventUpdateData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) all_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) date: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) due_date: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) participants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) external_participants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reminders: Option<Vec<Timestamp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rules: Option<Vec<RecurringRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exdate: Option<Vec<Timestamp>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventCreateData {
    pub(crate) calendar: String,
    pub(crate) event_id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    pub(crate) all_day: bool,
    pub(crate) date: Timestamp,
    pub(crate) due_date: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) participants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) external_participants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reminders: Option<Vec<Timestamp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rules: Option<Vec<RecurringRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exdate: Option<Vec<Timestamp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) original_start_time: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recurring_event_id: Option<String>,
    pub(crate) access: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HulyEventTx<'a, T> {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    #[serde(rename = "_class")]
    pub(crate) class: &'a str,
    pub(crate) space: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) modified_by: &'a str,
    pub(crate) object_id: &'a str,
    pub(crate) object_class: &'a str,
    pub(crate) object_space: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operations: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) attributes: Option<T>,
    pub(crate) collection: &'a str,
    pub(crate) attached_to: &'a str,
    pub(crate) attached_to_class: &'a str,
}

impl<'a, T> HulyEventTx<'a, T>
where
    T: serde::Serialize,
{
    pub(crate) fn pretty_str(&'a self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyAccount {
    pub(crate) uuid: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FindOptions {
    pub(crate) projection: Option<HashMap<&'static str, u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FindParams<'a> {
    #[serde(rename = "_class")]
    pub(crate) class: &'static str,
    pub(crate) query: HashMap<&'static str, &'a str>,
    pub(crate) options: Option<FindOptions>,
}

enum HttpMethod {
    Get,
    Post,
}

async fn api_call<'a, TResult, TParams>(
    method: HttpMethod,
    func: &str,
    user: &HulyUser,
    params: Option<&TParams>,
) -> Result<TResult, Error>
where
    TResult: for<'de> serde::de::Deserialize<'de>,
    TParams: serde::Serialize,
{
    let client = reqwest::Client::builder().gzip(true).build();
    if let Err(err) = client {
        return Err(Error::ApiError(format!("{} build_client: {}", func, err)));
    }
    let client = client.unwrap();
    let url = format!("{}/api/v1/{}/{}", user.api_url, func, user.workspace_uuid);
    let mut request = match method {
        HttpMethod::Get => client.get(url.clone()),
        HttpMethod::Post => client.post(url.clone()),
    }
    .header(AUTHORIZATION, format!("Bearer {}", user.token))
    .header(ACCEPT_ENCODING, "gzip");
    if let Some(params) = params {
        request = request.json(&params);
    }
    let response = request.send().await;
    if let Err(err) = response {
        //println!("*** API {}: http_error: {:?}", url, err);
        return Err(Error::ApiError(format!("{} http_error: {:?}", url, err)));
    }
    let response = response.unwrap();
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await;
        if let Err(err) = text {
            //println!("*** API {}: resp_error: {:?}", url, err);
            return Err(Error::ApiError(format!(
                "{} resp_error {}: {:?}",
                url, status, err
            )));
        }
        let text = text.unwrap();
        //println!("*** API {}: status_error: {:?}", url, text);
        return Err(Error::ApiError(format!(
            "{} status_error {}: {:?}",
            url, status, text
        )));
    }
    let text = response.text().await;
    if let Err(err) = text {
        //println!("*** API {}: resp_error: {:?}", url, err);
        return Err(Error::ApiError(format!("{} resp_error: {:?}", url, err)));
    }
    let text = text.unwrap();
    //println!("*** API {}: response_text: {:?}", url, text);
    let result = serde_json::from_str::<TResult>(&text);
    if let Err(err) = result {
        //println!("*** API {}: json_error: {:?}", url, err);
        return Err(Error::ApiError(format!("{} json_error: {:?}", url, err)));
    }
    Ok(result.unwrap())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindAllResult<T> {
    value: Vec<T>,
}

pub(crate) async fn find_all<'a, T>(
    user: &HulyUser,
    params: &FindParams<'a>,
) -> Result<Vec<T>, Error>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    let find_res: FindAllResult<T> =
        api_call(HttpMethod::Post, "find-all", user, Some(params)).await?;
    Ok(find_res.value)
}

#[derive(Debug, Deserialize)]
struct TxResult {}

async fn tx<'a, Tx>(user: &HulyUser, tx: &Tx) -> Result<(), Error>
where
    Tx: serde::Serialize,
{
    api_call::<TxResult, Tx>(HttpMethod::Post, "tx", user, Some(tx)).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedId {
    id: String,
}

pub(crate) async fn generate_id<'u>(user: &HulyUser) -> Result<String, Error> {
    let id =
        api_call::<GeneratedId, Option<()>>(HttpMethod::Get, "generate-id", user, None).await?;
    Ok(id.id)
}

pub(crate) async fn get_account(user: &HulyUser) -> Result<HulyAccount, Error> {
    let p: Option<&FindParams> = None;
    let account: HulyAccount = api_call(HttpMethod::Get, "account", user, p).await?;
    Ok(account)
}

pub(crate) async fn tx_create_event(
    user: &HulyUser,
    class: &str,
    data: &HulyEventCreateData,
) -> Result<(), Error> {
    let tx_id = generate_id(user).await?;
    let obj_id = generate_id(user).await?;
    let create_tx = HulyEventTx {
        id: tx_id,
        class: CLASS_TX_CREATE_DOC,
        space: SPACE_TX,
        modified_by: user.social_id.as_str(),
        created_by: user.social_id.as_str(),
        object_id: obj_id.as_str(),
        object_class: class,
        object_space: SPACE_CALENDAR,
        operations: None,
        attributes: Some(data),
        collection: COLLECTION_EVENTS,
        attached_to: ID_NOT_ATTACHED,
        attached_to_class: CLASS_EVENT,
    };
    println!(
        "*** CREATE_TX:\n{}",
        serde_json::to_string_pretty(&create_tx).unwrap()
    );
    tx(user, &create_tx).await
}

pub(crate) async fn tx_delete_event(user: &HulyUser, data: &HulyEventData) -> Result<(), Error> {
    let tx_id = generate_id(user).await?;
    let remove_tx = HulyEventTx::<()> {
        id: tx_id,
        class: CLASS_TX_REMOVE_DOC,
        space: SPACE_TX,
        modified_by: user.social_id.as_str(),
        created_by: user.social_id.as_str(),
        object_id: data.id.as_str(),
        object_class: data.class.as_str(),
        object_space: data.space.as_str(),
        operations: None,
        attributes: None,
        collection: data.collection.as_str(),
        attached_to: data.attached_to.as_str(),
        attached_to_class: data.attached_to_class.as_str(),
    };
    println!("*** REMOVE TX:\n{}", remove_tx.pretty_str());
    tx(user, &remove_tx).await
}

pub(crate) async fn tx_update_event(
    user: &HulyUser,
    old_event: &HulyEventData,
    data: &HulyEventUpdateData,
) -> Result<(), Error> {
    let tx_id = generate_id(user).await?;
    let update_tx = HulyEventTx {
        id: tx_id,
        class: CLASS_TX_UPDATE_DOC,
        space: SPACE_TX,
        modified_by: user.social_id.as_str(),
        created_by: user.social_id.as_str(),
        object_id: old_event.id.as_str(),
        object_class: old_event.class.as_str(),
        object_space: old_event.space.as_str(),
        operations: Some(data),
        attributes: None,
        collection: old_event.collection.as_str(),
        attached_to: old_event.attached_to.as_str(),
        attached_to_class: old_event.attached_to_class.as_str(),
    };
    println!("*** UPDATE_TX:\n{}", update_tx.pretty_str());
    tx(user, &update_tx).await
}
