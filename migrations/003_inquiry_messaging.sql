-- Migration: Inquiry Messaging System
-- Add messaging capability for inquiry negotiations

CREATE TABLE IF NOT EXISTS inquiry_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    inquiry_id UUID NOT NULL REFERENCES inquiries(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Indexes for performance
    CONSTRAINT inquiry_messages_message_not_empty CHECK (char_length(message) > 0)
);

CREATE INDEX idx_inquiry_messages_inquiry_id ON inquiry_messages(inquiry_id);
CREATE INDEX idx_inquiry_messages_sender_id ON inquiry_messages(sender_id);
CREATE INDEX idx_inquiry_messages_created_at ON inquiry_messages(created_at DESC);

-- Add negotiating status to inquiries
-- Update the existing status check constraint to include 'negotiating'
ALTER TABLE inquiries DROP CONSTRAINT IF EXISTS inquiries_status_check;
ALTER TABLE inquiries ADD CONSTRAINT inquiries_status_check
    CHECK (status IN ('pending', 'negotiating', 'accepted', 'rejected', 'converted_to_transaction'));

-- Add last_message_at column for sorting
ALTER TABLE inquiries ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP;

-- Create index on last_message_at for query performance
CREATE INDEX IF NOT EXISTS idx_inquiries_last_message_at ON inquiries(last_message_at DESC);

-- Function to update last_message_at when a new message is added
CREATE OR REPLACE FUNCTION update_inquiry_last_message_at()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE inquiries
    SET last_message_at = NEW.created_at
    WHERE id = NEW.inquiry_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically update last_message_at
DROP TRIGGER IF EXISTS trigger_update_inquiry_last_message_at ON inquiry_messages;
CREATE TRIGGER trigger_update_inquiry_last_message_at
    AFTER INSERT ON inquiry_messages
    FOR EACH ROW
    EXECUTE FUNCTION update_inquiry_last_message_at();

COMMENT ON TABLE inquiry_messages IS 'Chat messages for inquiry negotiations between buyers and sellers';
COMMENT ON COLUMN inquiry_messages.inquiry_id IS 'References the inquiry being discussed';
COMMENT ON COLUMN inquiry_messages.sender_id IS 'User who sent the message (buyer or seller)';
COMMENT ON COLUMN inquiry_messages.message IS 'Message content (cannot be empty)';
COMMENT ON COLUMN inquiries.last_message_at IS 'Timestamp of last message for sorting conversations';
