use super::HulyAuthProvider;
use crate::{
    account_api::{
        AccountClient, PersonId, PersonUuid, TokenClaims, WorkspaceUuid, generate_token,
    },
    api::{CLASS_PERSON, FindOptions, FindParams, HulyPerson, find_all},
};
use async_trait::async_trait;
use rustical_store::{
    Error,
    auth::{AuthenticationProvider, User},
};
use std::{collections::HashMap, time::SystemTime};

#[derive(Debug, Clone)]
pub(crate) struct HulyUser {
    pub(crate) id: String,
    pub(crate) contact_id: String,
    pub(crate) social_id: String,
    pub(crate) account_uuid: String,
    pub(crate) workspace_url: String,
    pub(crate) workspace_uuid: String,
    pub(crate) password: String,
    pub(crate) token: String,
    pub(crate) token_updated_at: SystemTime,
    pub(crate) api_url: String,
    pub(crate) remotes: Vec<String>,
    pub(crate) workspaces: Vec<WorkspaceInfo>,
}

struct LoginInfo {
    // Social id that is used for login to the CalDAV server
    social_id: PersonId,

    // Global person UUID corresponding to the social id
    account_uuid: PersonUuid,

    // Generated person token that is used for selecting workspace
    token: String,

    // List of workspace UUIDs for wich CalDAV integration is enabled for the social id
    workspaces: Vec<WorkspaceInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceInfo {
    pub(crate) uuid: WorkspaceUuid,
    pub(crate) url: String,
}

async fn login(
    url: &str,
    system_account_uuid: &str,
    secret: &str,
    user_id: &str,
    password: &str,
) -> Result<LoginInfo, Error> {
    let client = AccountClient::new_with_account(url, system_account_uuid, secret).await?;

    let social_id = client.find_social_id(user_id).await?;
    let account_uuid = client.find_account_uuid(&social_id).await?;

    let mut caldav_password = None;
    let mut caldav_workspaces = Vec::new();

    let integrations = client.list_integrations(&social_id).await?;
    for integration in &integrations {
        if integration.workspace_uuid.is_none()
            || integration.workspace_uuid.as_ref().unwrap().is_empty()
        {
            if caldav_password.is_none() {
                let integration_secret = client.get_integration_secret(&social_id).await?;
                caldav_password = Some(integration_secret.secret);
            }
        } else {
            caldav_workspaces.push(integration.workspace_uuid.as_ref().unwrap().to_string());
        }
    }

    if caldav_password.is_none() {
        return Err(Error::ApiError(format!(
            "CalDAV integration is not enabled for account {} (password)",
            user_id
        )));
    }
    let caldav_password = caldav_password.unwrap();

    if caldav_password != password {
        return Err(Error::ApiError("Provided password is incorrect".into()));
    }

    if caldav_workspaces.is_empty() {
        return Err(Error::ApiError(format!(
            "CalDAV integration is not enabled for account {} (workspaces)",
            user_id
        )));
    }

    let workspaces_info = client.get_workspaces_info(&caldav_workspaces).await?;
    let workspaces = workspaces_info
        .iter()
        .filter_map(|ws| {
            if ws.status.is_disabled || ws.status.mode != "active" {
                return None;
            }
            Some(WorkspaceInfo {
                uuid: ws.uuid.clone(),
                url: ws.url.clone(),
            })
        })
        .collect::<Vec<_>>();
    if workspaces.is_empty() {
        return Err(Error::ApiError(format!(
            "CalDAV integration is not enabled for account {} (active workspaces)",
            user_id
        )));
    }

    // This token will be used for selecting workspace
    let claims = TokenClaims {
        account: &account_uuid,
        ..Default::default()
    };
    let token = generate_token(&claims, secret)?;

    Ok(LoginInfo {
        account_uuid,
        token,
        social_id,
        workspaces,
    })
}

fn make_user(user: &HulyUser) -> User {
    User {
        id: user.id.clone(),
        displayname: Some(user.id.clone()),
        password: None,
        app_tokens: vec![],
        memberships: vec![],
        principal_type: rustical_store::auth::user::PrincipalType::Individual,
    }
}

impl HulyAuthProvider {
    async fn validate_huly_user(
        &self,
        addr_id_ws: &str,
        password: &str,
    ) -> Result<Option<User>, Error> {
        let mut p = addr_id_ws.split("|");
        let rem_addr = p.next().unwrap();
        let social_key = p.next().unwrap();
        let ws_url = p.next();

        tracing::debug!(
            "HULY_LOGIN rem_addr={} user_id={} ws_url={:?}",
            rem_addr,
            social_key,
            ws_url
        );

        let mut cache = self.calendar_cache.lock().await;

        // Handle login from different remote address
        if let Some(huly_user) = cache.try_get_user(social_key, ws_url) {
            let rem_addr = rem_addr.to_string();
            if !huly_user.remotes.contains(&rem_addr) {
                if password == &huly_user.password {
                    let mut huly_user = huly_user.clone();
                    huly_user.remotes.push(rem_addr);
                    cache.set_user(social_key, ws_url, huly_user);
                }
            }
        }

        // Handle token expiration
        if let Some(huly_user) = cache.try_get_user(social_key, ws_url) {
            if huly_user.token_updated_at.elapsed().unwrap() < self.token_expiration {
                return Ok(Some(make_user(&huly_user)));
            }
        }

        let login_info = login(
            &self.accounts_url,
            &self.system_account_uuid,
            &self.server_secret,
            social_key,
            password,
        )
        .await?;

        let mut huly_user = HulyUser {
            id: social_key.to_string(),
            contact_id: "".to_string(),
            social_id: login_info.social_id.clone(),
            account_uuid: login_info.account_uuid.clone(),
            workspace_url: "".to_string(),
            workspace_uuid: "".to_string(),
            password: password.to_string(),
            token: login_info.token.clone(),
            token_updated_at: SystemTime::now(),
            api_url: "".to_string(),
            remotes: vec![rem_addr.to_string()],
            workspaces: login_info.workspaces,
        };

        let Some(ws_url) = ws_url else {
            let user = make_user(&huly_user);
            cache.set_user(social_key, ws_url, huly_user);
            return Ok(Some(user));
        };

        let ws_uuid = huly_user
            .workspaces
            .iter()
            .find(|ws| ws.url == ws_url)
            .map(|ws| ws.uuid.clone());
        if ws_uuid.is_none() {
            return Err(Error::ApiError(format!(
                "CalDAV integration is not enabled for {} in workspace {}",
                social_key, ws_url
            )));
        }

        let account_client =
            AccountClient::new_with_token(&self.accounts_url, &login_info.token).await?;
        let selected_ws = account_client.select_workspace(ws_url).await?;
        huly_user.workspace_url = ws_url.to_string();
        huly_user.workspace_uuid = ws_uuid.unwrap();
        huly_user.token = selected_ws.0;
        huly_user.api_url = selected_ws.1;

        let params = FindParams {
            class: CLASS_PERSON,
            query: HashMap::from([("personUuid", login_info.account_uuid.as_str())]),
            options: Some(FindOptions {
                projection: Some(HashMap::from([("_id", 1)])),
                ..Default::default()
            }),
        };
        let persons = find_all::<HulyPerson>(&huly_user, &params).await?;
        if persons.is_empty() {
            return Err(Error::ApiError(format!(
                "No person found for account {}",
                login_info.account_uuid
            )));
        }
        let person = persons.get(0).unwrap();
        huly_user.contact_id = person.id.clone();

        println!("*** HULY_LOGIN_SUCCESS {:?}", huly_user);

        let user = make_user(&huly_user);
        cache.set_user(social_key, Some(ws_url), huly_user);
        Ok(Some(user))
    }
}

#[async_trait]
impl AuthenticationProvider for HulyAuthProvider {
    async fn get_principals(&self) -> Result<Vec<User>, Error> {
        return Ok(vec![]);
    }

    async fn get_principal(&self, principal: &str) -> Result<Option<User>, Error> {
        let cache = self.calendar_cache.lock().await;
        if let Some(huly_user) = cache.try_get_user(principal, None) {
            return Ok(Some(make_user(&huly_user)));
        }
        Ok(None)
    }

    async fn remove_principal(&self, _id: &str) -> Result<(), Error> {
        Ok(())
    }

    async fn insert_principal(&self, _user: User) -> Result<(), Error> {
        Ok(())
    }

    async fn validate_app_token(
        &self,
        addr_id_ws: &str,
        password: &str,
    ) -> Result<Option<User>, Error> {
        let res = self.validate_huly_user(addr_id_ws, password).await;
        if let Err(err) = &res {
            tracing::error!("Error validating user token: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let user = res.unwrap();
        Ok(user)
    }

    async fn validate_password(
        &self,
        addr_id_ws: &str,
        password: &str,
    ) -> Result<Option<User>, Error> {
        let res = self.validate_huly_user(addr_id_ws, password).await;
        if let Err(err) = &res {
            tracing::error!("Error validating user token: {:?}", err);
            // AuthenticationMiddleware can't handle errors, it crashes the request thread
            // Returning None will cause it to responce with 401 Unauthorized
            return Ok(None);
        }
        let user = res.unwrap();
        Ok(user)
    }

    async fn add_app_token(
        &self,
        _user_id: &str,
        name: String,
        _token: String,
    ) -> Result<String, Error> {
        return Ok(name);
    }

    async fn remove_app_token(&self, _user_id: &str, _token_id: &str) -> Result<(), Error> {
        Ok(())
    }
}
