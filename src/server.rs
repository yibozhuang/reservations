use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use reservations::reservations_server::{Reservations, ReservationsServer};
use reservations::{DeleteReservationRequest, ReservationRequest, ReservationResponse};
use reservations::{
    ListReservationRequest, ReservationOperationRequest, ReservationOperationResponse,
};

pub mod reservations {
    tonic::include_proto!("reservations");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("reservations_descriptor");
}

#[derive(Debug)]
struct ReservationsService;

#[tonic::async_trait]
impl Reservations for ReservationsService {
    async fn create_reservation(
        &self,
        _request: Request<ReservationOperationRequest>,
    ) -> Result<Response<ReservationOperationResponse>, Status> {
        unimplemented!()
    }

    async fn update_reservation(
        &self,
        _request: Request<ReservationOperationRequest>,
    ) -> Result<Response<ReservationOperationResponse>, Status> {
        unimplemented!()
    }

    async fn delete_reservation(
        &self,
        _request: Request<DeleteReservationRequest>,
    ) -> Result<Response<ReservationOperationResponse>, Status> {
        unimplemented!()
    }

    async fn get_reservation(
        &self,
        _request: Request<ReservationRequest>,
    ) -> Result<Response<ReservationResponse>, Status> {
        unimplemented!()
    }

    type ListReservationStream = ReceiverStream<Result<ReservationResponse, Status>>;

    async fn list_reservation(
        &self,
        _request: Request<ListReservationRequest>,
    ) -> Result<Response<Self::ListReservationStream>, Status> {
        unimplemented!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reflector = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(reservations::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let addr = "[::1]:10000".parse().unwrap();

    println!("Reservations server listening on: {}", addr);

    let reservations = ReservationsService {};

    let reservations_svc = ReservationsServer::new(reservations);

    Server::builder()
        .add_service(reflector)
        .add_service(reservations_svc)
        .serve(addr)
        .await?;

    Ok(())
}
