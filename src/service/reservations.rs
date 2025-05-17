use chrono::{DateTime, Utc};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::db::{Client as DbClient, RepositoryError, ReservationRepository};
use crate::proto::{
    reservation_service_server::ReservationService, Client as ProtoClient, ClientId, ClientList,
    ClientRequest, Reservation as ProtoReservation, ReservationId, ReservationList,
    ReservationRequest, SlotList, TimeRange, TimeSlot as ProtoTimeSlot,
};
use prost_types::Timestamp;

pub struct ReservationServiceImpl {
    repository: Arc<ReservationRepository>,
}

impl ReservationServiceImpl {
    pub fn new(repository: Arc<ReservationRepository>) -> Self {
        Self { repository }
    }

    fn timestamp_to_datetime(ts: &Timestamp) -> DateTime<Utc> {
        let seconds = ts.seconds;
        let nanos = ts.nanos as u32;
        DateTime::<Utc>::from_timestamp(seconds, nanos).unwrap_or(Utc::now())
    }

    fn datetime_to_timestamp(dt: &DateTime<Utc>) -> Timestamp {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }

    fn db_timeslot_to_proto(slot: &crate::db::TimeSlot) -> ProtoTimeSlot {
        ProtoTimeSlot {
            start_time: Some(Self::datetime_to_timestamp(&slot.start_time)),
            end_time: Some(Self::datetime_to_timestamp(&slot.end_time)),
        }
    }

    fn db_reservation_to_proto(res: &crate::db::Reservation) -> ProtoReservation {
        ProtoReservation {
            id: res.id.to_string(),
            client_id: res.client_id.to_string(),
            slot: Some(ProtoTimeSlot {
                start_time: Some(Self::datetime_to_timestamp(&res.start_time)),
                end_time: Some(Self::datetime_to_timestamp(&res.end_time)),
            }),
            created_at: Some(Self::datetime_to_timestamp(&res.created_at)),
            status: String::from(res.status.clone()),
            notes: res.notes.clone().unwrap_or_default(),
        }
    }

    fn db_client_to_proto(client: &DbClient) -> ProtoClient {
        ProtoClient {
            id: client.id.to_string(),
            name: client.name.clone(),
            email: client.email.clone(),
            created_at: Some(Self::datetime_to_timestamp(&client.created_at)),
        }
    }

    fn map_error(err: RepositoryError) -> Status {
        match err {
            RepositoryError::DatabaseError(e) => {
                tracing::error!("Database error: {:?}", e);
                Status::internal(format!("Internal error: {}", e))
            }
            RepositoryError::ReservationConflict => {
                Status::already_exists("The requested time slot is already booked")
            }
            RepositoryError::ReservationNotFound(id) => {
                Status::not_found(format!("Reservation not found with ID: {}", id))
            }
            RepositoryError::ClientNotFound(id) => {
                Status::not_found(format!("Client not found with ID: {}", id))
            }
        }
    }
}

#[tonic::async_trait]
impl ReservationService for ReservationServiceImpl {
    async fn list_available_slots(
        &self,
        request: Request<TimeRange>,
    ) -> Result<Response<SlotList>, Status> {
        let time_range = request.into_inner();

        let start_time = match time_range.start_time {
            Some(ts) => Self::timestamp_to_datetime(&ts),
            None => return Err(Status::invalid_argument("Start time is required")),
        };

        let end_time = match time_range.end_time {
            Some(ts) => Self::timestamp_to_datetime(&ts),
            None => return Err(Status::invalid_argument("End time is required")),
        };

        if start_time >= end_time {
            return Err(Status::invalid_argument(
                "Start time must be before end time",
            ));
        }

        let available_slots = self
            .repository
            .find_available_slots(start_time, end_time)
            .await
            .map_err(Self::map_error)?;

        let proto_slots = available_slots
            .iter()
            .map(|slot| Self::db_timeslot_to_proto(slot))
            .collect();

        Ok(Response::new(SlotList { slots: proto_slots }))
    }

    async fn create_reservation(
        &self,
        request: Request<ReservationRequest>,
    ) -> Result<Response<ProtoReservation>, Status> {
        let req = request.into_inner();

        // Parse client ID
        let client_id = req
            .client_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("Invalid client ID format"))?;

        // Parse time slot
        let slot = req
            .slot
            .ok_or(Status::invalid_argument("Time slot is required"))?;

        let start_time = match slot.start_time {
            Some(ts) => Self::timestamp_to_datetime(&ts),
            None => return Err(Status::invalid_argument("Start time is required")),
        };

        let end_time = match slot.end_time {
            Some(ts) => Self::timestamp_to_datetime(&ts),
            None => return Err(Status::invalid_argument("End time is required")),
        };

        if start_time >= end_time {
            return Err(Status::invalid_argument(
                "Start time must be before end time",
            ));
        }

        let notes = if req.notes.is_empty() {
            None
        } else {
            Some(req.notes.as_str())
        };
        let reservation = self
            .repository
            .create_reservation(client_id, start_time, end_time, notes)
            .await
            .map_err(Self::map_error)?;

        Ok(Response::new(Self::db_reservation_to_proto(&reservation)))
    }

    async fn get_reservation(
        &self,
        request: Request<ReservationId>,
    ) -> Result<Response<ProtoReservation>, Status> {
        let id = request
            .into_inner()
            .id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("Invalid reservation ID format"))?;

        let reservation = self
            .repository
            .get_reservation(id)
            .await
            .map_err(Self::map_error)?;

        Ok(Response::new(Self::db_reservation_to_proto(&reservation)))
    }

    async fn cancel_reservation(
        &self,
        request: Request<ReservationId>,
    ) -> Result<Response<()>, Status> {
        let id = request
            .into_inner()
            .id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("Invalid reservation ID format"))?;

        self.repository
            .cancel_reservation(id)
            .await
            .map_err(Self::map_error)?;

        Ok(Response::new(()))
    }

    async fn list_client_reservations(
        &self,
        request: Request<ClientId>,
    ) -> Result<Response<ReservationList>, Status> {
        let client_id = request
            .into_inner()
            .id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("Invalid client ID format"))?;

        let reservations = self
            .repository
            .get_client_reservations(client_id)
            .await
            .map_err(Self::map_error)?;

        let proto_reservations = reservations
            .iter()
            .map(|res| Self::db_reservation_to_proto(res))
            .collect();

        Ok(Response::new(ReservationList {
            reservations: proto_reservations,
        }))
    }

    async fn create_client(
        &self,
        request: Request<ClientRequest>,
    ) -> Result<Response<ProtoClient>, Status> {
        let req = request.into_inner();

        if req.name.is_empty() {
            return Err(Status::invalid_argument("Client name is required"));
        }

        if req.email.is_empty() {
            return Err(Status::invalid_argument("Client email is required"));
        }

        let client = self
            .repository
            .create_client(&req.name, &req.email)
            .await
            .map_err(Self::map_error)?;

        Ok(Response::new(Self::db_client_to_proto(&client)))
    }

    async fn list_clients(&self, _: Request<()>) -> Result<Response<ClientList>, Status> {
        let clients = self
            .repository
            .list_clients()
            .await
            .map_err(Self::map_error)?;

        let proto_clients = clients
            .iter()
            .map(|client| Self::db_client_to_proto(client))
            .collect();

        Ok(Response::new(ClientList {
            clients: proto_clients,
        }))
    }
}
