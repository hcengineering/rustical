use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rustical_store::{auth::User, Error};

pub(crate) type Timestamp = i64;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyCalendar {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    pub(crate) modified_on: Timestamp,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEvent {
    pub(crate) event_id: String,
    pub(crate) modified_on: Timestamp,
}

#[derive(Debug, Deserialize, Serialize)]
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
}

#[derive(Debug, Deserialize, Serialize, Default, Eq, PartialEq)]
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HulyEventTx<T> {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    #[serde(rename = "_class")]
    pub(crate) class: String,
    pub(crate) space: String,
    pub(crate) created_by: String,
    pub(crate) modified_by: String,
    pub(crate) object_id: String,
    pub(crate) object_class: String,
    pub(crate) object_space: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operations: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) attributes: Option<T>,
    pub(crate) collection: String,
    pub(crate) attached_to: String,
    pub(crate) attached_to_class: String
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
    pub(crate) projection: Option<HashMap<String, u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FindParams {
    #[serde(rename = "_class")]
    pub(crate) class: String,
    pub(crate) query: HashMap<String, String>,
    pub(crate) options: Option<FindOptions>,
}

// TODO: this is a temporary value living not longer than a User from which it's produced
// rewrite to use string refs with lifetime template args to avoid unnecessary cloning
pub(crate) struct ApiAuth {
    pub(crate) token: String,
    pub(crate) workspace: String,
}

impl TryFrom<&User> for ApiAuth {
    type Error = Error;

    fn try_from<'a>(user: &'a User) -> Result<Self, Self::Error> {
        let Some(workspace) = &user.workspace else {
            return Err(Error::ApiError("no workspace".into()))
        };
        let Some(token) = &user.password else {
            return Err(Error::ApiError("no token".into()))
        };
        Ok(Self {
            token: token.clone(),
            workspace: workspace.clone(),
        })
    }
}

enum HttpMethod {
    Get,
    Post,
}

async fn api_call<TResult, TParams>(
    url: &str, 
    method: HttpMethod, 
    func: &str, 
    auth: ApiAuth, 
    params: Option<TParams>
) -> Result<TResult, Error> 
where
    TResult: for<'a> serde::de::Deserialize<'a>,
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

pub(crate) async fn find_all<T>(url: &str, auth: ApiAuth, params: FindParams) -> Result<T, Error> 
where T: for<'a> serde::de::Deserialize<'a>,
{
    api_call(url, HttpMethod::Post, "find-all", auth, Some(params)).await
}

#[derive(Debug, Deserialize)]
struct TxResult {
}

pub(crate) async fn tx<Tx>(url: &str, auth: ApiAuth, tx: Tx) -> Result<(), Error> 
where
    Tx: serde::Serialize,
{
    _ = api_call::<TxResult, Tx>(url, HttpMethod::Post, "tx", auth, Some(tx)).await?;
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

pub(crate) async fn generate_id(url: &str, auth: ApiAuth, class: &str) -> Result<String, Error> {
    let p = Some(GenerateId { class: class.to_string() });
    let id: GeneratedId = api_call(url, HttpMethod::Post, "generate-id", auth, p).await?;
    Ok(id.id)
}

pub(crate) async fn get_account(url: &str, auth: ApiAuth) -> Result<HulyAccount, Error> {
    let p: Option<FindParams> = None;
    let account: HulyAccount = api_call(url, HttpMethod::Get, "account", auth, p).await?;
    Ok(account)
}
