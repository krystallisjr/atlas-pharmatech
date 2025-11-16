-- 🏛️ PRODUCTION REGULATORY DOCUMENT GENERATION SYSTEM
-- Supports: CoA (Certificate of Analysis), GDP (Good Distribution Practice), GMP (Good Manufacturing Practice)
-- Features: RAG with pgvector, Immutable Audit Ledger, Ed25519 Signatures

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

-- Enable vector extension for semantic search
CREATE EXTENSION IF NOT EXISTS vector;

-- Enable pgcrypto for digest/hash functions used in triggers
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- USER KEYPAIR MANAGEMENT (Ed25519 with libsodium)
-- ============================================================================

-- Add Ed25519 keypair columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS ed25519_public_key VARCHAR(64);  -- Hex-encoded 32-byte public key
ALTER TABLE users ADD COLUMN IF NOT EXISTS ed25519_private_key_encrypted TEXT;  -- AES-256-GCM encrypted private key
ALTER TABLE users ADD COLUMN IF NOT EXISTS keypair_generated_at TIMESTAMPTZ;

-- Index for public key lookups
CREATE INDEX IF NOT EXISTS idx_users_ed25519_public_key ON users(ed25519_public_key);

COMMENT ON COLUMN users.ed25519_public_key IS 'Ed25519 public key (hex) for document signing - regulatory compliance';
COMMENT ON COLUMN users.ed25519_private_key_encrypted IS 'AES-256-GCM encrypted Ed25519 private key - never expose in API';

-- ============================================================================
-- REGULATORY KNOWLEDGE BASE (RAG)
-- ============================================================================

CREATE TABLE IF NOT EXISTS regulatory_knowledge_base (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Document classification
    document_type VARCHAR(50) NOT NULL,  -- 'CoA', 'GDP', 'GMP', 'general'
    regulation_source VARCHAR(200),      -- 'FDA 21 CFR Part 211', 'EU GDP 2013/C 68/01', 'ICH Q7', etc.
    regulation_section VARCHAR(100),     -- '§211.160', 'Section 3.2', etc.

    -- Content
    section_title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,

    -- Vector embedding for semantic search (Claude embeddings are 1536 dimensions)
    embedding VECTOR(1536),

    -- Metadata for flexible storage
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Audit
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Vector similarity search index (IVFFlat for performance)
CREATE INDEX IF NOT EXISTS idx_knowledge_embedding ON regulatory_knowledge_base
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Traditional indexes
CREATE INDEX IF NOT EXISTS idx_knowledge_doc_type ON regulatory_knowledge_base(document_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_source ON regulatory_knowledge_base(regulation_source);
CREATE INDEX IF NOT EXISTS idx_knowledge_metadata ON regulatory_knowledge_base USING gin(metadata);

COMMENT ON TABLE regulatory_knowledge_base IS 'RAG knowledge base for regulatory document generation - FDA/EU/ICH regulations';
COMMENT ON COLUMN regulatory_knowledge_base.embedding IS 'Vector embedding for semantic similarity search';

-- ============================================================================
-- GENERATED REGULATORY DOCUMENTS
-- ============================================================================

CREATE TABLE IF NOT EXISTS regulatory_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Document classification
    document_type VARCHAR(50) NOT NULL,  -- 'CoA', 'GDP', 'GMP'
    document_number VARCHAR(100) NOT NULL UNIQUE,  -- e.g., 'COA-2025-001234'

    -- Content
    title VARCHAR(500) NOT NULL,
    content JSONB NOT NULL,  -- Structured document content
    content_markdown TEXT,   -- Markdown version for display
    content_hash VARCHAR(64) NOT NULL,  -- SHA-256 hash for integrity

    -- Associations
    product_id UUID REFERENCES pharmaceuticals(id) ON DELETE SET NULL,
    batch_number VARCHAR(100),
    inventory_id UUID REFERENCES inventory(id) ON DELETE SET NULL,

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'draft',  -- 'draft', 'pending_approval', 'approved', 'rejected', 'voided'

    -- Signatures
    generated_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    generated_signature VARCHAR(128),  -- Ed25519 signature by generator

    approved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    approved_signature VARCHAR(128),   -- Ed25519 signature by approver
    approved_at TIMESTAMPTZ,

    rejected_by UUID REFERENCES users(id) ON DELETE SET NULL,
    rejection_reason TEXT,
    rejected_at TIMESTAMPTZ,

    voided_by UUID REFERENCES users(id) ON DELETE SET NULL,
    void_reason TEXT,
    voided_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- RAG context used for generation
    rag_context JSONB,  -- Store the knowledge chunks used for context

    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_reg_docs_type ON regulatory_documents(document_type);
CREATE INDEX IF NOT EXISTS idx_reg_docs_number ON regulatory_documents(document_number);
CREATE INDEX IF NOT EXISTS idx_reg_docs_status ON regulatory_documents(status);
CREATE INDEX IF NOT EXISTS idx_reg_docs_product ON regulatory_documents(product_id);
CREATE INDEX IF NOT EXISTS idx_reg_docs_batch ON regulatory_documents(batch_number);
CREATE INDEX IF NOT EXISTS idx_reg_docs_generated_by ON regulatory_documents(generated_by);
CREATE INDEX IF NOT EXISTS idx_reg_docs_approved_by ON regulatory_documents(approved_by);
CREATE INDEX IF NOT EXISTS idx_reg_docs_created_at ON regulatory_documents(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_reg_docs_metadata ON regulatory_documents USING gin(metadata);

COMMENT ON TABLE regulatory_documents IS 'AI-generated regulatory documents (CoA, GDP, GMP) with Ed25519 signatures';
COMMENT ON COLUMN regulatory_documents.content_hash IS 'SHA-256 hash of content for integrity verification';
COMMENT ON COLUMN regulatory_documents.rag_context IS 'Knowledge base chunks used for RAG generation - for audit trail';

-- ============================================================================
-- IMMUTABLE AUDIT LEDGER (Blockchain-style)
-- ============================================================================

CREATE TABLE IF NOT EXISTS regulatory_document_ledger (
    id BIGSERIAL PRIMARY KEY,
    entry_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,

    -- Document reference
    document_id UUID NOT NULL REFERENCES regulatory_documents(id) ON DELETE RESTRICT,
    document_type VARCHAR(50) NOT NULL,

    -- Operation tracking
    operation VARCHAR(50) NOT NULL,  -- 'generated', 'approved', 'rejected', 'voided', 'amended'
    operation_description TEXT,

    -- Content integrity
    content_hash VARCHAR(64) NOT NULL,  -- SHA-256 of document content at this point in time
    content_snapshot JSONB,  -- Full document snapshot (for audit trail)

    -- Ed25519 Signature (libsodium)
    signature VARCHAR(128) NOT NULL,  -- Hex-encoded Ed25519 signature
    signature_public_key VARCHAR(64) NOT NULL,  -- Public key used for signing
    signature_algorithm VARCHAR(20) NOT NULL DEFAULT 'Ed25519',

    -- Actor information
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    user_email VARCHAR(255) NOT NULL,
    user_name VARCHAR(255),

    -- Blockchain-style chain hashing
    previous_entry_hash VARCHAR(64),  -- Hash of previous ledger entry (NULL for first entry)
    chain_hash VARCHAR(64) NOT NULL UNIQUE,  -- Hash of (previous_hash + content_hash + signature + timestamp)

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Network info for audit
    ip_address INET,
    user_agent TEXT,

    -- Immutable timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for querying
CREATE INDEX IF NOT EXISTS idx_ledger_document_id ON regulatory_document_ledger(document_id);
CREATE INDEX IF NOT EXISTS idx_ledger_user_id ON regulatory_document_ledger(user_id);
CREATE INDEX IF NOT EXISTS idx_ledger_operation ON regulatory_document_ledger(operation);
CREATE INDEX IF NOT EXISTS idx_ledger_created_at ON regulatory_document_ledger(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_chain_hash ON regulatory_document_ledger(chain_hash);

COMMENT ON TABLE regulatory_document_ledger IS 'Immutable audit ledger with Ed25519 signatures - blockchain-style chain hashing for regulatory compliance';
COMMENT ON COLUMN regulatory_document_ledger.chain_hash IS 'Blockchain-style hash linking to previous entry - ensures tamper-proof audit trail';

-- ============================================================================
-- IMMUTABILITY ENFORCEMENT
-- ============================================================================

-- Prevent updates to ledger entries (immutability)
CREATE OR REPLACE RULE regulatory_ledger_no_update AS
    ON UPDATE TO regulatory_document_ledger
    DO INSTEAD NOTHING;

-- Prevent deletes from ledger (immutability)
CREATE OR REPLACE RULE regulatory_ledger_no_delete AS
    ON DELETE TO regulatory_document_ledger
    DO INSTEAD NOTHING;

-- Trigger to calculate blockchain-style chain hash
CREATE OR REPLACE FUNCTION calculate_regulatory_chain_hash()
RETURNS TRIGGER AS $$
DECLARE
    prev_hash TEXT;
    data_to_hash TEXT;
BEGIN
    -- Get previous entry's chain hash (for blockchain linking)
    SELECT chain_hash INTO prev_hash
    FROM regulatory_document_ledger
    ORDER BY id DESC
    LIMIT 1;

    -- Combine: previous_hash + content_hash + signature + timestamp
    -- This creates a tamper-proof chain
    data_to_hash := COALESCE(prev_hash, '0000000000000000000000000000000000000000000000000000000000000000') ||
                   NEW.content_hash ||
                   NEW.signature ||
                   EXTRACT(EPOCH FROM NEW.created_at)::TEXT;

    -- Calculate SHA-256 hash
    NEW.previous_entry_hash := prev_hash;
    NEW.chain_hash := encode(digest(data_to_hash, 'sha256'), 'hex');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER regulatory_ledger_calculate_chain_hash
    BEFORE INSERT ON regulatory_document_ledger
    FOR EACH ROW
    EXECUTE FUNCTION calculate_regulatory_chain_hash();

-- ============================================================================
-- DOCUMENT TEMPLATES
-- ============================================================================

CREATE TABLE IF NOT EXISTS regulatory_document_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Template identification
    template_name VARCHAR(200) NOT NULL UNIQUE,
    document_type VARCHAR(50) NOT NULL,  -- 'CoA', 'GDP', 'GMP'
    version VARCHAR(50) NOT NULL,

    -- Template content
    template_structure JSONB NOT NULL,  -- JSON schema defining required fields
    validation_rules JSONB NOT NULL DEFAULT '{}',  -- Field validation rules

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Audit
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_templates_type ON regulatory_document_templates(document_type);
CREATE INDEX IF NOT EXISTS idx_templates_active ON regulatory_document_templates(is_active);

COMMENT ON TABLE regulatory_document_templates IS 'Document templates with validation rules for CoA, GDP, GMP';

-- ============================================================================
-- VIEWS FOR REPORTING
-- ============================================================================

-- View: Recent regulatory document activity
CREATE OR REPLACE VIEW recent_regulatory_activity AS
SELECT
    l.id,
    l.entry_id,
    l.operation,
    l.document_type,
    d.document_number,
    d.title,
    l.user_email,
    l.created_at,
    l.signature,
    l.chain_hash
FROM regulatory_document_ledger l
JOIN regulatory_documents d ON l.document_id = d.id
ORDER BY l.created_at DESC;

-- View: Document signatures summary
CREATE OR REPLACE VIEW regulatory_document_signatures AS
SELECT
    d.id AS document_id,
    d.document_number,
    d.document_type,
    d.status,
    d.generated_by,
    d.generated_signature,
    d.approved_by,
    d.approved_signature,
    d.approved_at,
    (d.approved_signature IS NOT NULL) AS is_signed,
    d.created_at
FROM regulatory_documents d;

-- View: Ledger chain integrity check
CREATE OR REPLACE VIEW ledger_chain_integrity AS
SELECT
    l1.id,
    l1.entry_id,
    l1.chain_hash,
    l1.previous_entry_hash,
    l2.chain_hash AS expected_previous_hash,
    (l1.previous_entry_hash = l2.chain_hash OR l1.previous_entry_hash IS NULL) AS chain_valid
FROM regulatory_document_ledger l1
LEFT JOIN regulatory_document_ledger l2 ON l2.id = (l1.id - 1)
ORDER BY l1.id;

COMMENT ON VIEW recent_regulatory_activity IS 'Recent regulatory document operations for audit dashboard';
COMMENT ON VIEW regulatory_document_signatures IS 'Summary of all document signatures for compliance reporting';
COMMENT ON VIEW ledger_chain_integrity IS 'Verify blockchain-style chain integrity - all should be chain_valid=true';

-- ============================================================================
-- INITIAL DATA: Document Number Sequences
-- ============================================================================

CREATE SEQUENCE IF NOT EXISTS coa_document_number_seq START WITH 1;
CREATE SEQUENCE IF NOT EXISTS gdp_document_number_seq START WITH 1;
CREATE SEQUENCE IF NOT EXISTS gmp_document_number_seq START WITH 1;

-- Function to generate document numbers
CREATE OR REPLACE FUNCTION generate_regulatory_document_number(doc_type VARCHAR)
RETURNS VARCHAR AS $$
DECLARE
    year VARCHAR(4);
    seq_num VARCHAR(10);
    prefix VARCHAR(10);
BEGIN
    year := EXTRACT(YEAR FROM CURRENT_DATE)::VARCHAR;

    CASE doc_type
        WHEN 'CoA' THEN
            prefix := 'COA';
            seq_num := LPAD(nextval('coa_document_number_seq')::TEXT, 6, '0');
        WHEN 'GDP' THEN
            prefix := 'GDP';
            seq_num := LPAD(nextval('gdp_document_number_seq')::TEXT, 6, '0');
        WHEN 'GMP' THEN
            prefix := 'GMP';
            seq_num := LPAD(nextval('gmp_document_number_seq')::TEXT, 6, '0');
        ELSE
            RAISE EXCEPTION 'Invalid document type: %', doc_type;
    END CASE;

    RETURN prefix || '-' || year || '-' || seq_num;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_regulatory_document_number IS 'Generate unique document numbers: COA-2025-000001, GDP-2025-000001, etc.';
