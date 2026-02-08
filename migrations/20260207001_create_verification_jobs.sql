-- Create verification_jobs table
CREATE TABLE IF NOT EXISTS verification_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status VARCHAR(20) NOT NULL CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    image_key VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retry_count INTEGER NOT NULL DEFAULT 0,
    error TEXT,

    -- Extracted fields (encrypted in application layer, stored as JSONB)
    extracted_fields JSONB,

    -- Validation results
    verification_result JSONB,

    -- Performance tracking
    processing_started_at TIMESTAMPTZ,
    processing_completed_at TIMESTAMPTZ,

    -- User/session tracking (for future authorization)
    user_id VARCHAR(100),

    CONSTRAINT retry_count_positive CHECK (retry_count >= 0)
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_jobs_status ON verification_jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON verification_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_user_id ON verification_jobs(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_image_key ON verification_jobs(image_key);

-- Create function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to auto-update updated_at
CREATE TRIGGER update_verification_jobs_updated_at
    BEFORE UPDATE ON verification_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add comment for documentation
COMMENT ON TABLE verification_jobs IS 'Tracks label verification jobs from upload through OCR processing to completion';
COMMENT ON COLUMN verification_jobs.image_key IS 'R2 storage key for the encrypted label image';
COMMENT ON COLUMN verification_jobs.extracted_fields IS 'OCR-extracted fields (encrypted in app, stored as JSONB)';
COMMENT ON COLUMN verification_jobs.verification_result IS 'TTB compliance validation results';
