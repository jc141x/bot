// Copyright 2022 jc141 (rokbma, noodle)
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::time::Duration;

use ruma::{
    api::client::{
        filter::FilterDefinition, membership::joined_members, message::send_message_event,
        sync::sync_events,
    },
    assign,
    events::{room::message::RoomMessageEventContent, AnySyncTimelineEvent, AnySyncStateEvent},
    TransactionId,
};

use tokio_stream::StreamExt as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    println!("Starting the bot...");
    let access_token: &'static str = env!("ACCESS_TOKEN");
    let room_id: &'static str = env!("ROOM_ID");
    let homeserver_url: &'static str = env!("HOMESERVER_URL");
    let http_client = ruma::client::http_client::Reqwest::new();
    let client = ruma::client::Client::builder()
        .homeserver_url(homeserver_url.to_string())
        .access_token(Some(access_token.into(),
        ))
        .http_client(http_client)
        .await?;

    let filter = FilterDefinition::ignore_all().into();
    let initial_sync_response = client
        .send_request(assign!(sync_events::v3::Request::new(), {
            filter: Some(&filter),
        }))
        .await?;

    let lobby = room_id.try_into().unwrap();

    let mut members = client
        .send_request(joined_members::v3::Request::new(lobby))
        .await?
        .joined;
    println!("Current members: {:?}", members.len());

    let mut sync_stream = Box::pin(client.sync(
        None,
        initial_sync_response.next_batch,
        &ruma::presence::PresenceState::Online,
        Some(Duration::from_secs(30)),
    ));

    println!(" Listening to new member events.");

    while let Some(res) = sync_stream.try_next().await? {
        for (room_id, room) in res.rooms.join {
            for event in room
                .timeline
                .events
                .into_iter()
                .flat_map(|r| r.deserialize())
            {
                if let AnySyncTimelineEvent::State(AnySyncStateEvent::RoomMember(change)) = &event {
                    let _membership_change = change.membership();
                    let sender = event.sender();
                    println!("{} triggered a join event", event.sender());
                    let new_members = client
                        .send_request(joined_members::v3::Request::new(lobby))
                        .await?
                        .joined;
                    println!(
                        "Member change from: {}, to: {}",
                        members.len(),
                        new_members.len()
                    );
                    if new_members.len() > members.len() {
                        println!("Greeting {}", sender);
                        let message = format!(
                            "Welcome {} where do you download our torrents from?",
                            sender
                        );
                        client
                            .send_request(send_message_event::v3::Request::new(
                                &room_id,
                                &TransactionId::new(),
                                &RoomMessageEventContent::text_plain(message),
                            )?)
                            .await?;
                    }
                    members = new_members;
                }
            }
        }
    }

    Ok(())
}
