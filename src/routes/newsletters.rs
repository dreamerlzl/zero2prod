use std::sync::Arc;

use anyhow::anyhow;
use poem::{web::Json, Endpoint};
use poem_openapi::{Object, OpenApi, OpenApiService};
use sea_orm::{
    entity::*, ColumnTrait, DeriveColumn, EntityTrait, EnumIter, QueryFilter, QuerySelect,
};
use serde::Deserialize;

use super::{add_tracing, subscriptions::ConfirmStatus};
use crate::{
    context::StateContext,
    domain::Email,
    entities::subscriptions::{self, Entity as Subscriptions},
    routes::ApiErrorResponse,
};

pub struct Api {
    context: Arc<StateContext>,
}

pub fn get_api_service(
    context: Arc<StateContext>,
    server_url: &str,
) -> (OpenApiService<Api, ()>, impl Endpoint) {
    let api_service =
        OpenApiService::new(Api::new(context), "newsletters", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

#[OpenApi]
impl Api {
    #[oai(path = "/", method = "post", transform = "add_tracing")]
    async fn publish_newsletter(
        &self,
        body: Json<NewsletterJsonPost>,
    ) -> Result<(), ApiErrorResponse> {
        // list all confirmed subscribers
        // ideally, we should let some workers to handle all the confirmed subscribers
        // asynchrounously
        let subscribers = self.get_confirmed_subscribers().await?;
        for subscriber in subscribers {
            match subscriber {
                Ok(subscriber) => {
                    self.context
                        .email_client
                        .send_email(
                            &subscriber.email,
                            &body.title,
                            &body.content.html,
                            &body.content.text,
                        )
                        .await?;
                }
                Err(error) => {
                    tracing::warn!(error.cause_chain=?error, "skipping a confirmed subscriber due to invalid email stored")
                }
            }
        }
        Ok(())
    }
}

// {
//   "title": "abc",
//   "content": {
//     "html": "xxx",
//     "text": "yyy",
//   }
// }
#[derive(Deserialize, Debug, Object)]
struct NewsletterJsonPost {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize, Object)]
struct Content {
    html: String,
    text: String,
}

impl Api {
    fn new(context: Arc<StateContext>) -> Self {
        Api { context }
    }

    #[tracing::instrument(name = "get confirmed subscribers", skip(self))]
    async fn get_confirmed_subscribers(
        &self,
    ) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sea_orm::DbErr> {
        // select only one column without using a struct; ugly
        #[derive(Debug, Copy, Clone, EnumIter, DeriveColumn)]
        enum QueryAs {
            Email,
        }
        let subscribers = Subscriptions::find()
            .filter(subscriptions::Column::Status.eq(ConfirmStatus::Confirmed.to_string()))
            .select_only()
            .column(subscriptions::Column::Email)
            .into_values::<_, QueryAs>()
            .all(&self.context.db)
            .await?
            // when we first store the subscribers' email, the app could be version X
            // when we later fetch and parse the email, the app could be version Y
            // email validation logic may change between these 2 versions
            .into_iter()
            .map(|r| match Email::parse(r) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(err) => Err(anyhow!(err)),
            })
            .collect();
        Ok(subscribers)
    }
}

struct ConfirmedSubscriber {
    email: Email,
}
