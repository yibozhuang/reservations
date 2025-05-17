use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};
use uuid::Uuid;

/// Represents a client in the system
#[derive(Debug, Clone)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl FromRow<'_, PgRow> for Client {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Client {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// Status of a reservation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationStatus {
    Confirmed,
    Cancelled,
}

impl From<String> for ReservationStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "cancelled" => ReservationStatus::Cancelled,
            _ => ReservationStatus::Confirmed,
        }
    }
}

impl From<ReservationStatus> for String {
    fn from(status: ReservationStatus) -> Self {
        match status {
            ReservationStatus::Confirmed => "confirmed".to_string(),
            ReservationStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

/// Represents a reservation in the database
#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: Uuid,
    pub client_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: ReservationStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl FromRow<'_, PgRow> for Reservation {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let status: String = row.try_get("status")?;

        Ok(Reservation {
            id: row.try_get("id")?,
            client_id: row.try_get("client_id")?,
            start_time: row.try_get("start_time")?,
            end_time: row.try_get("end_time")?,
            status: ReservationStatus::from(status),
            notes: row.try_get("notes")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// Represents a time slot
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}
