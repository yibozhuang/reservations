-- Create reservation database schema

-- Table to store clients
CREATE TABLE clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Table to store time slots
CREATE TABLE reservations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID NOT NULL REFERENCES clients(id),
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'confirmed',
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure end time is after start time
    CONSTRAINT valid_time_range CHECK (end_time > start_time),
    
    -- Create an index to help with searching by time ranges
    CONSTRAINT no_overlapping_reservations EXCLUDE USING gist (
        tstzrange(start_time, end_time) WITH &&
    )
);

-- Create index for searching reservations by client
CREATE INDEX idx_reservations_client_id ON reservations(client_id);

-- Create index for searching reservations by time range
CREATE INDEX idx_reservations_time_range ON reservations USING gist (tstzrange(start_time, end_time));

-- Create index for status to quickly filter active reservations
CREATE INDEX idx_reservations_status ON reservations(status);