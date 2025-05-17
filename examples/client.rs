use chrono::{Duration, TimeZone, Utc};
use prost_types::Timestamp;
use tonic::Request;

pub mod proto {
    tonic::include_proto!("reservations");
}

use proto::reservation_service_client::ReservationServiceClient;
use proto::{ClientId, ClientRequest, ReservationId, ReservationRequest, TimeRange};

fn datetime_to_timestamp(dt: &chrono::DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn prost_timestamp_to_human_readable(ts: &Timestamp) -> String {
    let dt = Utc
        .timestamp_opt(ts.seconds, ts.nanos as u32)
        .single()
        .expect("Invalid timestamp");

    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const NAME: &str = "Foo Bar";
    const EMAIL: &str = "foo-bar@example.com";

    let mut client = ReservationServiceClient::connect("http://[::1]:50051").await?;

    println!("\n--- Setting up client ---");
    // First check if there is already an existing client
    let response = client.list_clients(Request::new(())).await?;
    let clients = response.into_inner().clients;
    let mut client_id = String::new();
    for client in clients {
        if client.name == NAME {
            println!(
                "Found existing client: ID={}, Name={}",
                client.id, client.name
            );
            client_id = client.id.clone();
            break;
        }
    }

    if client_id.is_empty() {
        let client_request = Request::new(ClientRequest {
            name: NAME.to_string(),
            email: EMAIL.to_string(),
        });

        let response = client.create_client(client_request).await?;
        let client_info = response.into_inner();
        client_id = client_info.id.clone();
        println!(
            "Created client: ID={}, Name={}, Email={}",
            client_info.id, client_info.name, client_info.email
        );
    }

    // List available slots
    let now = Utc::now();
    let tomorrow = now + Duration::days(1);

    println!("\n--- Looking for available slots ---");
    let request = Request::new(TimeRange {
        start_time: Some(datetime_to_timestamp(&now)),
        end_time: Some(datetime_to_timestamp(&tomorrow)),
    });

    let response = client.list_available_slots(request).await?;
    let slots = response.into_inner().slots;
    println!("Found {} available slots", slots.len());

    // Create a reservation (using the first available slot)
    println!("\n--- Creating a reservation ---");
    for slot in &slots {
        let client_id = client_id.clone();
        let slot = slot.clone();

        let request = Request::new(ReservationRequest {
            client_id: client_id.clone(),
            slot: Some(slot),
            notes: "Example reservation".to_string(),
        });

        let response = match client.create_reservation(request).await {
            Ok(res) => res,
            Err(e) => {
                if e.code() != tonic::Code::AlreadyExists {
                    eprintln!("Error creating reservation: {:?}", e);
                }
                continue;
            }
        };

        let reservation = response.into_inner();
        println!(
            "Created reservation: ID={}, From={}, To={}, Status={}",
            reservation.id,
            prost_timestamp_to_human_readable(
                reservation
                    .slot
                    .clone()
                    .unwrap()
                    .start_time
                    .as_ref()
                    .unwrap()
            ),
            prost_timestamp_to_human_readable(
                reservation.slot.clone().unwrap().end_time.as_ref().unwrap()
            ),
            reservation.status
        );

        // Get reservation details
        println!("\n--- Getting reservation details ---");
        let request = Request::new(ReservationId {
            id: reservation.id.clone(),
        });

        let response = client.get_reservation(request).await?;
        let reservation = response.into_inner();
        println!(
            "Reservation details: ID={}, From={}, To={}, Status={}",
            reservation.id,
            prost_timestamp_to_human_readable(
                reservation
                    .slot
                    .clone()
                    .unwrap()
                    .start_time
                    .as_ref()
                    .unwrap()
            ),
            prost_timestamp_to_human_readable(
                reservation.slot.clone().unwrap().end_time.as_ref().unwrap()
            ),
            reservation.status
        );

        // List client reservations
        println!("\n--- Listing client reservations ---");
        let request = Request::new(ClientId {
            id: client_id.clone(),
        });

        let response = client.list_client_reservations(request).await?;
        let reservations = response.into_inner().reservations;

        println!("Client has {} reservations:", reservations.len());
        for (i, res) in reservations.iter().enumerate() {
            println!(
                "  Reservation #{}: ID={}, Status={}",
                i + 1,
                res.id,
                res.status
            );
        }

        // Cancel a reservation
        println!("\n--- Cancelling reservation ---");
        let request = Request::new(ReservationId {
            id: reservation.clone().id,
        });

        let _ = client.cancel_reservation(request).await?;
        println!("Reservation {} cancelled successfully!", reservation.id);

        // Verify the cancellation
        println!("\n--- Listing client reservations after cancellation ---");
        let request = Request::new(ClientId { id: client_id });

        let response = client.list_client_reservations(request).await?;
        let reservations = response.into_inner().reservations;

        for res in reservations {
            println!("  Reservation ID={}, Status={}", res.id, res.status);
        }

        break;
    }
    Ok(())
}
