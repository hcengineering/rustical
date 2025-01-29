use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rustical_store::{auth::User, Error};

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
    pub(crate) reminders: Option<Vec<Timestamp>>,
    pub(crate) time_zone: Option<String>,
    pub(crate) rules: Option<Vec<RecurringRule>>,
    pub(crate) exdate: Option<Vec<Timestamp>>,
    pub(crate) is_cancelled: Option<bool>,
    pub(crate) original_start_time: Option<Timestamp>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEvent {
    pub(crate) data: HulyEventData,
    pub(crate) instances: Option<Vec<HulyEventData>>,
}

#[derive(Debug, Deserialize, Serialize, Default, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecurringRule {
    pub(crate) freq: String,
    pub(crate) end_date: Option<Timestamp>,
    pub(crate) count: Option<u32>,
    pub(crate) interval: Option<u32>,
    pub(crate) by_second: Option<Vec<u8>>,
    pub(crate) by_minute: Option<Vec<u8>>,
    pub(crate) by_hour: Option<Vec<u8>>,
    pub(crate) by_day: Option<Vec<String>>,
    pub(crate) by_month_day: Option<Vec<u8>>,
    pub(crate) by_year_day: Option<Vec<u16>>,
    pub(crate) by_week_no: Option<Vec<i8>>,
    pub(crate) by_month: Option<Vec<u8>>,
    pub(crate) by_set_pos: Option<Vec<i16>>,
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
    pub(crate) date: Timestamp,
    pub(crate) due_date: Timestamp,
    pub(crate) description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) participants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reminders: Option<Vec<Timestamp>>,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    pub(crate) all_day: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_zone: Option<String>,
    pub(crate) access: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rules: Option<Vec<RecurringRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exdate: Option<Vec<Timestamp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) original_start_time: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recurring_event_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventTx<'a, T> {
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
    pub(crate) attached_to_class: &'a str
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyAccount {
    #[serde(rename = "_id")]
    pub(crate) id: String,
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

pub(crate) struct ApiAuth<'a> {
    pub(crate) token: &'a str,
    pub(crate) workspace: &'a str,
}

impl<'a> TryFrom<&'a User> for ApiAuth<'a> {
    type Error = Error;

    fn try_from(user: &'a User) -> Result<Self, Self::Error> {
        let Some(workspace) = &user.workspace else {
            return Err(Error::ApiError("no workspace".into()))
        };
        let Some(token) = &user.password else {
            return Err(Error::ApiError("no token".into()))
        };
        Ok(Self {
            token: token.as_str(),
            workspace: workspace.as_str(),
        })
    }
}

enum HttpMethod {
    Get,
    Post,
}

async fn api_call<'a, TResult, TParams>(
    url: &str, 
    method: HttpMethod, 
    func: &str, 
    auth: &ApiAuth<'a>, 
    params: Option<&TParams>
) -> Result<TResult, Error> 
where
    TResult: for<'de> serde::de::Deserialize<'de>,
    TParams: serde::Serialize,
{
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/{}/{}", url, func, auth.workspace);
    let mut request = 
        match method {
            HttpMethod::Get => client.get(url),
            HttpMethod::Post => client.post(url),
        }
        .header("Authorization", format!("Bearer {}", auth.token));
    if let Some(params) = params {
        request = request.json(&params);
    }
    let response = request
        .send()
        .await;
    if let Err(err) = response {
        //println!("*** API {}: http_error: {:?}", func, err);
        return Err(Error::ApiError(format!("{} http_error: {:?}", func, err)));
    }
    let response = response.unwrap();
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await;
        if let Err(err) = text {
            //println!("*** API {}: resp_error: {:?}", func, err);
            return Err(Error::ApiError(format!("{} resp_error {}: {:?}", func, status, err)));
        }
        let text = text.unwrap();
        //println!("*** API {}: status_error: {:?}", func, text);
        return Err(Error::ApiError(format!("{} status_error {}: {:?}", func, status,text)));
    }
    let text = response.text().await;
    if let Err(err) = text {
        //println!("*** API {}: resp_error: {:?}", func, err);
        return Err(Error::ApiError(format!("{} resp_error: {:?}", func, err)));
    }
    let text = text.unwrap();
    // println!("*** response_text: {:?}", text);
    let result = serde_json::from_str::<TResult>(&text);
    if let Err(err) = result {
        //println!("*** API {}: json_error: {:?}", func, err);
        return Err(Error::ApiError(format!("{} json_error: {:?}", func, err)));
    }
    Ok(result.unwrap())
}

pub(crate) async fn find_all<'a, T>(url: &str, auth: &ApiAuth<'a>, params: &FindParams<'a>) -> Result<T, Error> 
where T: for<'de> serde::de::Deserialize<'de>,
{
    api_call(url, HttpMethod::Post, "find-all", auth, Some(params)).await
}

#[derive(Debug, Deserialize)]
struct TxResult {
}

pub(crate) async fn tx<'a, Tx>(url: &str, auth: &ApiAuth<'a>, tx: &Tx) -> Result<(), Error> 
where
    Tx: serde::Serialize,
{
    api_call::<TxResult, Tx>(url, HttpMethod::Post, "tx", auth, Some(tx)).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedId {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateId {
    #[serde(rename = "_class")]
    class: String,
}

pub(crate) async fn generate_id<'u>(url: &str, auth: &ApiAuth<'u>, class: &str) -> Result<String, Error> {
    let p = GenerateId { class: class.to_string() };
    let id: GeneratedId = api_call(url, HttpMethod::Post, "generate-id", auth, Some(&p)).await?;
    Ok(id.id)
}

pub(crate) async fn get_account<'u>(url: &str, auth: &ApiAuth<'u>) -> Result<HulyAccount, Error> {
    let p: Option<&FindParams> = None;
    let account: HulyAccount = api_call(url, HttpMethod::Get, "account", auth, p).await?;
    Ok(account)
}

pub(crate) async fn tx_create_event(url: &str, user: &User, class: &str, data: &HulyEventCreateData) -> Result<(), Error> {
    let auth = user.try_into()?;
    let account = user.account.as_ref().map(|s| s.as_str()).ok_or_else(|| Error::InvalidData("Missing user account id".into()))?;
    let tx_id = generate_id(url, &auth, CLASS_TX_CREATE_DOC).await?;
    let obj_id = generate_id(url, &auth, class).await?;
    let create_tx = HulyEventTx {
        id: tx_id,
        class: CLASS_TX_CREATE_DOC,
        space: SPACE_TX,
        modified_by: account,
        created_by: account,
        object_id: obj_id.as_str(),
        object_class: class,
        object_space: SPACE_CALENDAR,
        operations: None,
        attributes: Some(data),
        collection: COLLECTION_EVENTS,
        attached_to: ID_NOT_ATTACHED,
        attached_to_class: CLASS_EVENT,
    };
    println!("*** CREATE_TX {}", serde_json::to_string_pretty(&create_tx).unwrap());
    tx(url, &auth, &create_tx).await?;
    Ok(())
}
