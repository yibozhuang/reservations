use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use super::models::{Client, Reservation, TimeSlot};

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Reservation conflict: The requested time slot is already booked")]
    ReservationConflict,

    #[error("Reservation not found with ID: {0}")]
    ReservationNotFound(Uuid),

    #[error("Client not found with ID: {0}")]
    ClientNotFound(Uuid),
}

pub struct ReservationRepository {
    pool: PgPool,
}

impl ReservationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_client(&self, name: &str, email: &str) -> Result<Client, RepositoryError> {
        let client = sqlx::query_as::<_, Client>(
            "INSERT INTO clients (name, email) VALUES ($1, $2) RETURNING *",
        )
        .bind(name)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(client)
    }

    pub async fn list_clients(&self) -> Result<Vec<Client>, RepositoryError> {
        let clients = sqlx::query_as::<_, Client>("SELECT * FROM clients")
            .fetch_all(&self.pool)
            .await?;

        Ok(clients)
    }

    pub async fn is_slot_available(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reservations 
             WHERE status = 'confirmed' 
             AND tstzrange($1, $2) && tstzrange(start_time, end_time)",
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 == 0)
    }

    pub async fn find_available_slots(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<TimeSlot>, RepositoryError> {
        let existing_reservations = sqlx::query_as::<_, Reservation>(
            "SELECT * FROM reservations 
             WHERE status = 'confirmed' 
             AND tstzrange(start_time, end_time) && tstzrange($1, $2)
             ORDER BY start_time",
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut available_slots = Vec::new();
        let mut current_time = start_date;

        while current_time < end_date {
            let slot_end = current_time + chrono::Duration::hours(1);

            // Check if this slot overlaps with any existing reservation
            let is_available = !existing_reservations.iter().any(|res| {
                (current_time >= res.start_time && current_time < res.end_time)
                    || (slot_end > res.start_time && slot_end <= res.end_time)
                    || (current_time <= res.start_time && slot_end >= res.end_time)
            });

            if is_available {
                available_slots.push(TimeSlot {
                    start_time: current_time,
                    end_time: slot_end,
                });
            }

            current_time = slot_end;
        }

        Ok(available_slots)
    }

    pub async fn create_reservation(
        &self,
        client_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        notes: Option<&str>,
    ) -> Result<Reservation, RepositoryError> {
        // Start a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // Check if client exists
        let client_exists = sqlx::query("SELECT 1 FROM clients WHERE id = $1")
            .bind(client_id)
            .fetch_optional(&mut *tx)
            .await?
            .is_some();

        if !client_exists {
            return Err(RepositoryError::ClientNotFound(client_id));
        }

        // Try to create the reservation
        // The database constraint will prevent overlapping reservations
        let result = self
            .create_reservation_tx(&mut tx, client_id, start_time, end_time, notes)
            .await;

        match result {
            Ok(reservation) => {
                // Commit the transaction
                tx.commit().await?;
                Ok(reservation)
            }
            Err(err) => {
                // Rollback on error
                let _ = tx.rollback().await;

                // Check if this was a conflict error
                if let RepositoryError::DatabaseError(sqlx::Error::Database(ref db_err)) = err {
                    if db_err.constraint() == Some("no_overlapping_reservations") {
                        return Err(RepositoryError::ReservationConflict);
                    }
                }

                Err(err)
            }
        }
    }

    /// Helper function to create a reservation within a transaction
    async fn create_reservation_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        client_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        notes: Option<&str>,
    ) -> Result<Reservation, RepositoryError> {
        let reservation = sqlx::query_as::<_, Reservation>(
            "INSERT INTO reservations (client_id, start_time, end_time, notes)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(client_id)
        .bind(start_time)
        .bind(end_time)
        .bind(notes)
        .fetch_one(&mut **tx)
        .await?;

        Ok(reservation)
    }

    /// Get a reservation by ID
    pub async fn get_reservation(&self, id: Uuid) -> Result<Reservation, RepositoryError> {
        let reservation =
            sqlx::query_as::<_, Reservation>("SELECT * FROM reservations WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(RepositoryError::ReservationNotFound(id))?;

        Ok(reservation)
    }

    /// Cancel a reservation
    pub async fn cancel_reservation(&self, id: Uuid) -> Result<(), RepositoryError> {
        let rows_affected = sqlx::query(
            "UPDATE reservations SET status = 'cancelled' WHERE id = $1 AND status = 'confirmed'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            // Check if the reservation exists
            let exists = sqlx::query("SELECT 1 FROM reservations WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .is_some();

            if !exists {
                return Err(RepositoryError::ReservationNotFound(id));
            }
            // If it exists but wasn't updated, it was already cancelled
        }

        Ok(())
    }

    /// Get all reservations for a client
    pub async fn get_client_reservations(
        &self,
        client_id: Uuid,
    ) -> Result<Vec<Reservation>, RepositoryError> {
        // Check if client exists
        let client_exists = sqlx::query("SELECT 1 FROM clients WHERE id = $1")
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

        if !client_exists {
            return Err(RepositoryError::ClientNotFound(client_id));
        }

        let reservations = sqlx::query_as::<_, Reservation>(
            "SELECT * FROM reservations WHERE client_id = $1 ORDER BY start_time",
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(reservations)
    }
}
