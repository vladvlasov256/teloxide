#![allow(clippy::large_enum_variant)]
use serde::{de::MapAccess, Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::types::{
    CallbackQuery, Chat, ChatJoinRequest, ChatMemberUpdated, ChosenInlineResult, InlineQuery,
    Message, Poll, PollAnswer, PreCheckoutQuery, ShippingQuery, User,
};

/// This [object] represents an incoming update.
///
/// [The official docs](https://core.telegram.org/bots/api#update).
///
/// [object]: https://core.telegram.org/bots/api#available-types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Update {
    /// The update‘s unique identifier. Update identifiers start from a certain
    /// positive number and increase sequentially. This ID becomes especially
    /// handy if you’re using webhooks, since it allows you to ignore
    /// repeated updates or to restore the correct update sequence, should
    /// they get out of order. If there are no new updates for at least a
    /// week, then identifier of the next update will be chosen randomly
    /// instead of sequentially.
    #[serde(rename = "update_id")]
    pub id: i32,

    #[serde(flatten)]
    pub kind: UpdateKind,
}

impl Update {
    // FIXME: add mentioned_users -> impl Iterator<&User>

    /// Returns the user that performed the action that caused this update, if
    /// known.
    ///
    /// This is generally the `from` field (except for `PollAnswer` where it's
    /// `user` and `Poll` with `Error` which don't have such field at all).
    #[must_use]
    pub fn from(&self) -> Option<&User> {
        use UpdateKind::*;

        let from = match &self.kind {
            Message(m) | EditedMessage(m) | ChannelPost(m) | EditedChannelPost(m) => m.from()?,

            CallbackQuery(query) => &query.from,
            ChosenInlineResult(chosen) => &chosen.from,
            InlineQuery(query) => &query.from,
            ShippingQuery(query) => &query.from,
            PreCheckoutQuery(query) => &query.from,
            PollAnswer(answer) => &answer.user,

            MyChatMember(m) | ChatMember(m) => &m.from,
            ChatJoinRequest(r) => &r.from,

            Poll(_) | Error(_) => return None,
        };

        Some(from)
    }

    /// Returns the chat in which is update has happened, if any.
    #[must_use]
    pub fn chat(&self) -> Option<&Chat> {
        use UpdateKind::*;

        let chat = match &self.kind {
            Message(m) | EditedMessage(m) | ChannelPost(m) | EditedChannelPost(m) => &m.chat,
            CallbackQuery(q) => &q.message.as_ref()?.chat,
            ChatMember(m) => &m.chat,
            MyChatMember(m) => &m.chat,
            ChatJoinRequest(c) => &c.chat,

            InlineQuery(_)
            | ChosenInlineResult(_)
            | ShippingQuery(_)
            | PreCheckoutQuery(_)
            | Poll(_)
            | PollAnswer(_)
            | Error(_) => return None,
        };

        Some(chat)
    }

    #[deprecated(note = "renamed to `from`", since = "0.13.0")]
    pub fn user(&self) -> Option<&User> {
        self.from()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateKind {
    // NB: When adding new variants, don't forget to update
    //     - `AllowedUpdate`
    //     - `Update::user`
    //     - `Update::chat`
    //     - `DpHandlerDescription::full_set`
    //     - `dispatching/filter_ext.rs`
    /// New incoming message of any kind — text, photo, sticker, etc.
    Message(Message),

    /// New version of a message that is known to the bot and was edited.
    EditedMessage(Message),

    /// New incoming channel post of any kind — text, photo, sticker, etc.
    ChannelPost(Message),

    /// New version of a channel post that is known to the bot and was edited.
    EditedChannelPost(Message),

    /// New incoming [inline] query.
    ///
    /// [inline]: https://core.telegram.org/bots/api#inline-mode
    InlineQuery(InlineQuery),

    /// The result of an [inline] query that was chosen by a user and sent to
    /// their chat partner. Please see our documentation on the [feedback
    /// collecting] for details on how to enable these updates for your bot.
    ///
    /// [inline]: https://core.telegram.org/bots/api#inline-mode
    /// [feedback collecting]: https://core.telegram.org/bots/inline#collecting-feedback
    ChosenInlineResult(ChosenInlineResult),

    /// New incoming callback query.
    CallbackQuery(CallbackQuery),

    /// New incoming shipping query. Only for invoices with flexible price.
    ShippingQuery(ShippingQuery),

    /// New incoming pre-checkout query. Contains full information about
    /// checkout.
    PreCheckoutQuery(PreCheckoutQuery),

    /// New poll state. Bots receive only updates about stopped polls and
    /// polls, which are sent by the bot.
    Poll(Poll),

    /// A user changed their answer in a non-anonymous poll. Bots receive new
    /// votes only in polls that were sent by the bot itself.
    PollAnswer(PollAnswer),

    /// The bot's chat member status was updated in a chat. For private chats,
    /// this update is received only when the bot is blocked or unblocked by the
    /// user.
    MyChatMember(ChatMemberUpdated),

    /// A chat member's status was updated in a chat. The bot must be an
    /// administrator in the chat and must explicitly specify
    /// [`AllowedUpdate::ChatMember`] in the list of `allowed_updates` to
    /// receive these updates.
    ///
    /// [`AllowedUpdate::ChatMember`]: crate::types::AllowedUpdate::ChatMember
    ChatMember(ChatMemberUpdated),

    /// A request to join the chat has been sent. The bot must have the
    /// can_invite_users administrator right in the chat to receive these
    /// updates.
    ChatJoinRequest(ChatJoinRequest),

    /// An error that happened during deserialization.
    ///
    /// This allows `teloxide` to continue working even if telegram adds a new
    /// kind of updates.
    Error(Value),
}

impl<'de> Deserialize<'de> for UpdateKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UpdateKind;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut tmp = None;

                // Try to deserialize a borrowed-str key, or else try deserializing an owned
                // string key
                let k = map.next_key::<&str>().or_else(|_| {
                    map.next_key::<String>().map(|k| {
                        tmp = k;
                        tmp.as_deref()
                    })
                });

                if let Ok(Some(k)) = k {
                    let res = match k {
                        "message" => {
                            map.next_value::<Message>().map(UpdateKind::Message).map_err(|_| false)
                        }
                        "edited_message" => map
                            .next_value::<Message>()
                            .map(UpdateKind::EditedMessage)
                            .map_err(|_| false),
                        "channel_post" => map
                            .next_value::<Message>()
                            .map(UpdateKind::ChannelPost)
                            .map_err(|_| false),
                        "edited_channel_post" => map
                            .next_value::<Message>()
                            .map(UpdateKind::EditedChannelPost)
                            .map_err(|_| false),
                        "inline_query" => map
                            .next_value::<InlineQuery>()
                            .map(UpdateKind::InlineQuery)
                            .map_err(|_| false),
                        "chosen_inline_result" => map
                            .next_value::<ChosenInlineResult>()
                            .map(UpdateKind::ChosenInlineResult)
                            .map_err(|_| false),
                        "callback_query" => map
                            .next_value::<CallbackQuery>()
                            .map(UpdateKind::CallbackQuery)
                            .map_err(|_| false),
                        "shipping_query" => map
                            .next_value::<ShippingQuery>()
                            .map(UpdateKind::ShippingQuery)
                            .map_err(|_| false),
                        "pre_checkout_query" => map
                            .next_value::<PreCheckoutQuery>()
                            .map(UpdateKind::PreCheckoutQuery)
                            .map_err(|_| false),
                        "poll" => map.next_value::<Poll>().map(UpdateKind::Poll).map_err(|_| false),
                        "poll_answer" => map
                            .next_value::<PollAnswer>()
                            .map(UpdateKind::PollAnswer)
                            .map_err(|_| false),
                        "my_chat_member" => map
                            .next_value::<ChatMemberUpdated>()
                            .map(UpdateKind::MyChatMember)
                            .map_err(|_| false),
                        "chat_member" => map
                            .next_value::<ChatMemberUpdated>()
                            .map(UpdateKind::ChatMember)
                            .map_err(|_| false),
                        "chat_join_request" => map
                            .next_value::<ChatJoinRequest>()
                            .map(UpdateKind::ChatJoinRequest)
                            .map_err(|_| false),

                        _ => Err(true),
                    };

                    let value_available = match res {
                        Ok(ok) => return Ok(ok),
                        Err(e) => e,
                    };

                    let mut value = serde_json::Map::new();
                    value.insert(
                        k.to_owned(),
                        if value_available {
                            map.next_value::<Value>().unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        },
                    );

                    while let Ok(Some((k, v))) = map.next_entry::<_, Value>() {
                        value.insert(k, v);
                    }

                    return Ok(UpdateKind::Error(Value::Object(value)));
                }

                Ok(UpdateKind::Error(Value::Object(<_>::default())))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl Serialize for UpdateKind {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = "UpdateKind";
        match self {
            UpdateKind::Message(v) => s.serialize_newtype_variant(name, 0, "message", v),
            UpdateKind::EditedMessage(v) => {
                s.serialize_newtype_variant(name, 1, "edited_message", v)
            }
            UpdateKind::ChannelPost(v) => s.serialize_newtype_variant(name, 2, "channel_post", v),
            UpdateKind::EditedChannelPost(v) => {
                s.serialize_newtype_variant(name, 3, "edited_channel_post", v)
            }
            UpdateKind::InlineQuery(v) => s.serialize_newtype_variant(name, 4, "inline_query", v),
            UpdateKind::ChosenInlineResult(v) => {
                s.serialize_newtype_variant(name, 5, "chosen_inline_result", v)
            }
            UpdateKind::CallbackQuery(v) => {
                s.serialize_newtype_variant(name, 6, "callback_query", v)
            }
            UpdateKind::ShippingQuery(v) => {
                s.serialize_newtype_variant(name, 7, "shipping_query", v)
            }
            UpdateKind::PreCheckoutQuery(v) => {
                s.serialize_newtype_variant(name, 8, "pre_checkout_query", v)
            }
            UpdateKind::Poll(v) => s.serialize_newtype_variant(name, 9, "poll", v),
            UpdateKind::PollAnswer(v) => s.serialize_newtype_variant(name, 10, "poll_answer", v),
            UpdateKind::MyChatMember(v) => {
                s.serialize_newtype_variant(name, 11, "my_chat_member", v)
            }
            UpdateKind::ChatMember(v) => s.serialize_newtype_variant(name, 12, "chat_member", v),
            UpdateKind::ChatJoinRequest(v) => {
                s.serialize_newtype_variant(name, 13, "chat_join_request", v)
            }
            UpdateKind::Error(v) => v.serialize(s),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::types::{
        Chat, ChatId, ChatKind, ChatPrivate, MediaKind, MediaText, Message, MessageCommon,
        MessageId, MessageKind, Update, UpdateKind, User, UserId,
    };

    use chrono::{DateTime, NaiveDateTime, Utc};

    // TODO: more tests for deserialization
    #[test]
    fn message() {
        let timestamp = 1_569_518_342;
        let date =
            DateTime::from_utc(NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(), Utc);

        let json = r#"{
            "update_id":892252934,
            "message":{
                "message_id":6557,
                "from":{
                    "id":218485655,
                    "is_bot": false,
                    "first_name":"Waffle",
                    "username":"WaffleLapkin",
                    "language_code":"en"
                },
                "chat":{
                    "id":218485655,
                    "first_name":"Waffle",
                    "username":"WaffleLapkin",
                    "type":"private"
                },
               "date":1569518342,
               "text":"hello there"
            }
        }"#;

        let expected = Update {
            id: 892_252_934,
            kind: UpdateKind::Message(Message {
                via_bot: None,
                id: MessageId(6557),
                thread_id: None,
                date,
                chat: Chat {
                    id: ChatId(218_485_655),
                    kind: ChatKind::Private(ChatPrivate {
                        username: Some(String::from("WaffleLapkin")),
                        first_name: Some(String::from("Waffle")),
                        last_name: None,
                        bio: None,
                        has_private_forwards: None,
                        has_restricted_voice_and_video_messages: None,
                        emoji_status_custom_emoji_id: None,
                    }),
                    photo: None,
                    pinned_message: None,
                    message_auto_delete_time: None,
                    has_hidden_members: false,
                    has_aggressive_anti_spam_enabled: false,
                },
                kind: MessageKind::Common(MessageCommon {
                    from: Some(User {
                        id: UserId(218_485_655),
                        is_bot: false,
                        first_name: String::from("Waffle"),
                        last_name: None,
                        username: Some(String::from("WaffleLapkin")),
                        language_code: Some(String::from("en")),
                        is_premium: false,
                        added_to_attachment_menu: false,
                    }),
                    reply_to_message: None,
                    forward: None,
                    edit_date: None,
                    media_kind: MediaKind::Text(MediaText {
                        text: String::from("hello there"),
                        entities: vec![],
                    }),
                    reply_markup: None,
                    sender_chat: None,
                    author_signature: None,
                    is_topic_message: false,
                    is_automatic_forward: false,
                    has_protected_content: false,
                }),
            }),
        };

        let actual = serde_json::from_str::<Update>(json).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn de_private_chat_text_message() {
        let text = r#"
  {
    "message": {
      "chat": {
        "first_name": "Hirrolot",
        "id": 408258968,
        "type": "private",
        "username": "hirrolot"
      },
      "date": 1581448857,
      "from": {
        "first_name": "Hirrolot",
        "id": 408258968,
        "is_bot": false,
        "language_code": "en",
        "username": "hirrolot"
      },
      "message_id": 154,
      "text": "4"
    },
    "update_id": 306197398
  }
"#;

        let Update { kind, .. } = serde_json::from_str::<Update>(text).unwrap();
        match kind {
            UpdateKind::Message(_) => {}
            _ => panic!("Expected `Message`"),
        }
    }

    #[test]
    fn pinned_message_works() {
        let json = r#"{
    "message": {
        "chat": {
            "id": -1001276785818,
            "title": "teloxide dev",
            "type": "supergroup",
            "username": "teloxide_dev"
        },
        "date": 1582134655,
        "from": {
            "first_name": "Hirrolot",
            "id": 408258968,
            "is_bot": false,
            "username": "hirrolot"
        },
        "message_id": 20225,
        "pinned_message": {
            "chat": {
                "id": -1001276785818,
                "title": "teloxide dev",
                "type": "supergroup",
                "username": "teloxide_dev"
            },
            "date": 1582134643,
            "from": {
                "first_name": "Hirrolot",
                "id": 408258968,
                "is_bot": false,
                "username": "hirrolot"
            },
            "message_id": 20224,
            "text": "Faster than a bullet"
        }
    },
    "update_id": 845402291
}"#;

        let Update { kind, .. } = serde_json::from_str(json).unwrap();
        match kind {
            UpdateKind::Message(_) => {}
            _ => panic!("Expected `Message`"),
        }
    }

    #[test]
    fn dice_works() {
        let json = r#"
        {
    "message": {
        "chat": {
            "id": -1001276785818,
            "title": "bla bla bla chat",
            "type": "supergroup",
            "username": "teloxide_dev"
        },
        "date": 1596014550,
        "dice": {
            "emoji": "🎲",
            "value": 2
        },
        "from": {
            "first_name": "Hirrolot",
            "id": 408258968,
            "is_bot": false,
            "language_code": "en",
            "username": "hirrolot"
        },
        "message_id": 35410
    },
    "update_id": 573255266
}
        "#;

        let Update { kind, .. } = serde_json::from_str(json).unwrap();
        match kind {
            UpdateKind::Message(_) => {}
            _ => panic!("Expected `Message`"),
        }
    }

    #[test]
    fn new_update_kind_error() {
        let json = r#"{
            "new_update_kind": {"some_field_idk": 1},
            "update_id": 1
        }"#;

        let Update { kind, .. } = serde_json::from_str(json).unwrap();

        match kind {
            // Deserialization failed successfully
            UpdateKind::Error(_) => {}
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn issue_523() {
        let json = r#"{
            "update_id":0,
            "my_chat_member": {
                "chat":{"id":0,"first_name":"FN","last_name":"LN","username":"UN","type":"private"},
                "from":{"id":0,"is_bot":false,"first_name":"FN","last_name":"LN","username":"UN"},
                "date":1644677726,
                "old_chat_member":{"user":{"id":1,"is_bot":true,"first_name":"bot","username":"unBot"},"status":"member"},
                "new_chat_member":{"user":{"id":1,"is_bot":true,"first_name":"bot","username":"unBot"},"status":"kicked","until_date":0}
            }
        }"#;

        let Update { kind, .. } = serde_json::from_str(json).unwrap();

        match kind {
            UpdateKind::MyChatMember(_) => {}
            _ => panic!("Expected `MyChatMember`"),
        }
    }
}
