// 🏛️ PRODUCTION KNOWLEDGE BASE SEEDING TOOL
// Populates regulatory_knowledge_base with FDA/EU/ICH regulations using Claude embeddings
// Usage: cargo run --bin seed_knowledge_base --release

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;

#[derive(Debug, Clone)]
struct RegulatoryEntry {
    document_type: String,
    regulation_source: String,
    regulation_section: String,
    section_title: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🏛️ Regulatory Knowledge Base Seeding Tool");
    tracing::info!("==========================================");

    // Load environment
    dotenv::dotenv().ok();

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/atlas_pharma".to_string());

    // Get API key
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow!("ANTHROPIC_API_KEY not set in .env"))?;

    // Connect to database
    tracing::info!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;

    // Check current count
    let current_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM regulatory_knowledge_base")
        .fetch_one(&pool)
        .await?;

    tracing::info!("Current knowledge base entries: {}", current_count);

    // Get all regulatory entries
    let entries = get_regulatory_content();
    tracing::info!("Total entries to seed: {}", entries.len());

    // Initialize embedding service
    tracing::info!("Initializing Claude embedding service...");

    // Create a system user for seeding (or use existing admin)
    let system_user_id: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM users WHERE email = 'test@encrypted.com' LIMIT 1"
    )
    .fetch_one(&pool)
    .await?;

    // Import the embedding service (we'll use the actual service)
    let embedding_service = atlas_pharma::services::ClaudeEmbeddingService::new(
        pool.clone(),
        api_key,
        system_user_id,
    )?;

    // Process in batches of 10 to avoid rate limits
    let batch_size = 10;
    let total_batches = (entries.len() + batch_size - 1) / batch_size;

    tracing::info!("Processing {} batches of {}", total_batches, batch_size);

    for (batch_idx, chunk) in entries.chunks(batch_size).enumerate() {
        tracing::info!(
            "Processing batch {}/{} ({} entries)...",
            batch_idx + 1,
            total_batches,
            chunk.len()
        );

        for entry in chunk {
            // Generate embedding for this content
            let embedding = embedding_service
                .generate_embedding(&entry.content)
                .await
                .map_err(|e| anyhow!("Failed to generate embedding: {}", e))?;

            // Insert into database
            sqlx::query!(
                r#"
                INSERT INTO regulatory_knowledge_base (
                    document_type,
                    regulation_source,
                    regulation_section,
                    section_title,
                    content,
                    embedding,
                    metadata,
                    created_by
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT DO NOTHING
                "#,
                entry.document_type,
                entry.regulation_source,
                entry.regulation_section,
                entry.section_title,
                entry.content,
                embedding as _,
                serde_json::json!({}),
                system_user_id
            )
            .execute(&pool)
            .await?;

            tracing::info!("  ✓ Seeded: {} - {}", entry.regulation_section, entry.section_title);
        }

        // Small delay between batches to avoid rate limits
        if batch_idx < total_batches - 1 {
            tracing::info!("Waiting 2 seconds before next batch...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    // Final count
    let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM regulatory_knowledge_base")
        .fetch_one(&pool)
        .await?;

    tracing::info!("==========================================");
    tracing::info!("✅ Seeding complete!");
    tracing::info!("   Before: {} entries", current_count);
    tracing::info!("   After:  {} entries", final_count);
    tracing::info!("   Added:  {} entries", final_count - current_count);

    Ok(())
}

/// Get comprehensive FDA/EU/ICH regulatory content
fn get_regulatory_content() -> Vec<RegulatoryEntry> {
    let mut entries = Vec::new();

    // ========================================================================
    // FDA 21 CFR Part 211 - GMP for Finished Pharmaceuticals
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.194".to_string(),
        section_title: "Laboratory Records".to_string(),
        content: "Complete records shall be maintained of any testing and standardization of laboratory reference standards, reagents, and standard solutions. Such records shall include the name of the reference standard, reagent, or solution; the manufacturer's or supplier's name and lot number; the date of receipt; the tests performed and date tests performed; and the results and conclusions derived therefrom.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.160".to_string(),
        section_title: "General Requirements (Laboratory Controls)".to_string(),
        content: "The establishment of any specifications, standards, sampling plans, test procedures, or other laboratory control mechanisms required by this subpart, including any change in such specifications, standards, sampling plans, test procedures, or other laboratory control mechanisms, shall be drafted by the appropriate organizational unit and reviewed and approved by the quality control unit. The requirements in this subpart shall be followed and shall be documented at the time of performance. Any deviation from the written specifications, standards, sampling plans, test procedures, or other laboratory control mechanisms shall be recorded and justified.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.165".to_string(),
        section_title: "Testing and Release for Distribution".to_string(),
        content: "For each batch of drug product, there shall be appropriate laboratory determination of satisfactory conformance to final specifications for the drug product, including the identity and strength of each active ingredient, prior to release. Where sterility and/or pyrogen testing are conducted on specific batches of shortlived radiopharmaceuticals, such batches may be released prior to completion of sterility and/or pyrogen testing, provided such testing is completed as soon as possible.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.166".to_string(),
        section_title: "Stability Testing".to_string(),
        content: "There shall be a written testing program designed to assess the stability characteristics of drug products. The results of such stability testing shall be used in determining appropriate storage conditions and expiration dates. The written program shall be followed and shall include: Sample size and test intervals based on statistical criteria for each attribute examined to assure valid estimates of stability; Storage conditions for samples retained for testing; Reliable, meaningful, and specific test methods; Testing of the drug product in the same container-closure system as that in which the drug product is marketed; Testing of drug products for reconstitution at the time of dispensing (as directed in the labeling) as well as after they are reconstituted.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GMP".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.100".to_string(),
        section_title: "Written Procedures; Deviations".to_string(),
        content: "There shall be written procedures for production and process control designed to assure that the drug products have the identity, strength, quality, and purity they purport or are represented to possess. Such procedures shall include all requirements in this subpart. These written procedures, including any changes, shall be drafted, reviewed, and approved by the appropriate organizational units and reviewed and approved by the quality control unit.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GMP".to_string(),
        regulation_source: "FDA 21 CFR Part 211".to_string(),
        regulation_section: "§211.110".to_string(),
        section_title: "Sampling and Testing of In-Process Materials and Drug Products".to_string(),
        content: "Control procedures shall be established to monitor the output and to validate the performance of those manufacturing processes that may be responsible for causing variability in the characteristics of in-process material and the drug product. Such control procedures shall include, but are not limited to, the following, where appropriate: Determination of conformance to written specifications for the identity, strength, quality, and purity of in-process materials and drug products; Calibration of instruments, apparatus, gauges, and recording devices at suitable intervals in accordance with an established written program containing specific directions, schedules, limits for accuracy and precision, and provisions for remedial action in the event accuracy and/or precision limits are not met.".to_string(),
    });

    // ========================================================================
    // EU GDP Guidelines 2013/C 68/01
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "GDP".to_string(),
        regulation_source: "EU GDP Guidelines 2013/C 68/01".to_string(),
        regulation_section: "Section 3.2".to_string(),
        section_title: "Temperature and Environment Control".to_string(),
        content: "Medicinal products should be transported in such a way that they are maintained within acceptable temperature limits. Where other storage conditions are required, such as protection from light, these should also be maintained. Equipment used for temperature monitoring during transport within vehicles and/or containers should be maintained and calibrated at regular intervals at least once a year. For products requiring specific temperature storage conditions, qualified equipment should be used to ensure correct storage conditions are maintained during transportation. Records of temperatures during transport should be available for review.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GDP".to_string(),
        regulation_source: "EU GDP Guidelines 2013/C 68/01".to_string(),
        regulation_section: "Section 4.1".to_string(),
        section_title: "Qualification of Suppliers and Customers".to_string(),
        content: "Wholesale distributors should obtain their supplies of medicinal products only from persons who are themselves in possession of a wholesale distribution authorisation, or who are in possession of a manufacturing authorisation. The quality system should include a procedure for qualifying suppliers and customers in accordance with the risk management process. Distributors should monitor and periodically evaluate their suppliers to confirm that they continue to meet the required standards.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GDP".to_string(),
        regulation_source: "EU GDP Guidelines 2013/C 68/01".to_string(),
        regulation_section: "Section 5.2".to_string(),
        section_title: "Storage Conditions and Premises".to_string(),
        content: "Medicinal products should be stored in segregated areas which are clearly marked and to which access is restricted to authorised persons. Any system replacing physical segregation, such as computerised systems, should provide equivalent security and should be validated. Storage areas should be designed or adapted to ensure good storage conditions. In particular, they should be clean, dry and maintained within acceptable temperature limits. Where special storage conditions are required (e.g. temperature, humidity) these should be provided, checked, monitored and recorded.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GDP".to_string(),
        regulation_source: "EU GDP Guidelines 2013/C 68/01".to_string(),
        regulation_section: "Section 9.1".to_string(),
        section_title: "Documentation and Records".to_string(),
        content: "All documents should be designed, prepared, reviewed and distributed with care. They should comply with the relevant parts of applicable product specifications and regulatory marketing and manufacturing authorisations. Documents should be approved, signed and dated by appropriate and authorised persons. Documents should have unambiguous contents and be uniquely identifiable. The layout should be orderly and easy to check. Documents should be regularly reviewed and kept up to date. When a document has been revised, systems should be in place to prevent inadvertent use of superseded documents.".to_string(),
    });

    // ========================================================================
    // ICH Q7 - GMP for Active Pharmaceutical Ingredients
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "GMP".to_string(),
        regulation_source: "ICH Q7".to_string(),
        regulation_section: "Section 11.1".to_string(),
        section_title: "General Controls for Laboratory Operations".to_string(),
        content: "General laboratory controls should include scientifically sound and appropriate specifications, standards, sampling plans, and test procedures designed to assure that components, intermediates, APIs, and labels and packaging materials conform to established standards of identity, strength, quality, and purity. Laboratory controls should include establishment of scientifically sound and appropriate specifications, standards, sampling plans, and test procedures; sampling of starting materials, packaging materials, intermediates, and APIs according to approved procedures; qualification of test methods; testing performed by qualified personnel; and comparison of analytical results against established acceptance criteria.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GMP".to_string(),
        regulation_source: "ICH Q7".to_string(),
        regulation_section: "Section 11.4".to_string(),
        section_title: "Certificates of Analysis".to_string(),
        content: "Certificates of Analysis should be issued for each batch of intermediate and API, as appropriate. Information on the batch should include: The name of the intermediate or API including where appropriate the grade; The batch number; The date of release; The expiration or retest date; A statement of compliance to the specification; Results of testing; Information on the manufacturing unit including specific identification of the manufacturing site; and Original signature of the person releasing the batch with the date of signature.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "ICH Q7".to_string(),
        regulation_section: "Section 11.7".to_string(),
        section_title: "Retention of Laboratory Records and Samples".to_string(),
        content: "Laboratory control records should be retained for at least the lifetime of the API batch. These records should include: Complete data derived from all tests conducted to ensure compliance with established specifications and standards, including examinations and assays; A record of all laboratory sampling, testing, and analysis; Records of all stability testing performed; A record of the receipt of reference standards and reagents used in tests; and Records of calibration of laboratory instruments, apparatus, gauges, and recording devices.".to_string(),
    });

    // ========================================================================
    // ICH Q6A - Specifications for New Drug Substances and Products
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "ICH Q6A".to_string(),
        regulation_section: "Section 2.1".to_string(),
        section_title: "General Considerations for Setting Specifications".to_string(),
        content: "A specification is defined as a list of tests, references to analytical procedures, and appropriate acceptance criteria that are numerical limits, ranges, or other criteria for the test described. It establishes the set of criteria to which a new drug substance or new drug product should conform to be considered acceptable for its intended use. Conformance to specification means that the drug substance and drug product, when tested according to the listed analytical procedures, will meet the listed acceptance criteria.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "ICH Q6A".to_string(),
        regulation_section: "Section 3.2".to_string(),
        section_title: "Universally Applicable Tests for Drug Products".to_string(),
        content: "The following tests are considered, in most cases, necessary for all new drug products: Description, Identification, Assay (content or potency), Impurities, and Dissolution (solid oral dosage forms). Additional tests may be required depending on the dosage form and the route of administration. For sterile products, tests for sterility and bacterial endotoxins or pyrogens are required. The specification for a drug product should include a suitable description of the product.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "ICH Q6A".to_string(),
        regulation_section: "Section 4.1".to_string(),
        section_title: "Acceptance Criteria".to_string(),
        content: "Acceptance criteria should be established and justified based on data obtained during development, pre-clinical and clinical studies, data from stability studies, relevant data from marketed products, and data from the scientific literature. Acceptance criteria should be achievable with the proposed method(s) and should reflect what is achievable with a robust commercial manufacturing process. Tighter specifications may be appropriate for individual batches and this may be particularly important for biotechnological/biological products where process changes can have a significant impact on product quality.".to_string(),
    });

    // ========================================================================
    // USP Reference Standards for Common Pharmaceuticals
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "USP General Chapter <905>".to_string(),
        regulation_section: "<905>".to_string(),
        section_title: "Uniformity of Dosage Units".to_string(),
        content: "Unless otherwise stated in the individual monograph, the requirements for Uniformity of Dosage Units are met if the acceptance value of the first 10 dosage units is less than or equal to L1% (15.0). If the acceptance value is greater than L1%, test the next 20 dosage units and calculate the acceptance value. The requirements are met if the final acceptance value of the 30 dosage units is less than or equal to L1% and no individual content of any dosage unit is less than (1 - L2 × 0.01)M or more than (1 + L2 × 0.01)M. The procedure is applicable to solid, semi-solid, and liquid dosage forms.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "USP General Chapter <711>".to_string(),
        regulation_section: "<711>".to_string(),
        section_title: "Dissolution".to_string(),
        content: "Dissolution test is a requirement for all solid oral dosage forms, designed to measure the rate and extent of dissolution of the active pharmaceutical ingredient(s). The test involves placing the dosage unit in a vessel containing a specified volume of dissolution medium (usually 900 mL) maintained at 37±0.5°C. The vessel is equipped with a stirring element (paddle or basket) rotating at a specified speed. Samples are withdrawn at specified time intervals and analyzed. Acceptance criteria are expressed as Q%, the amount of dissolved active ingredient specified in the individual monograph, expressed as a percentage of the labeled content.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "USP General Chapter <467>".to_string(),
        regulation_section: "<467>".to_string(),
        section_title: "Residual Solvents".to_string(),
        content: "Residual solvents are organic volatile chemicals that are used or produced in the manufacture of drug substances, excipients, or drug products. These solvents are classified into three classes based on their toxicity: Class 1 (solvents to be avoided), Class 2 (solvents to be limited), and Class 3 (solvents with low toxic potential). Appropriate test procedures should be employed to determine residual solvents. Gas chromatography is the preferred method due to its precision and accuracy. The acceptance criteria should be based on the permitted daily exposure (PDE) for each solvent.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "USP General Chapter <61>".to_string(),
        regulation_section: "<61>".to_string(),
        section_title: "Microbiological Examination of Nonsterile Products".to_string(),
        content: "Microbiological examination of nonsterile products consists of the enumeration of mesophilic bacteria and fungi that may grow under aerobic conditions, and detection of specified microorganisms that should be absent from the product. The total aerobic microbial count (TAMC) and total combined yeasts/molds count (TYMC) are determined. For oral dosage forms, the typical acceptance criteria are: TAMC not more than 10³ CFU/g or mL, TYMC not more than 10² CFU/g or mL, absence of Escherichia coli in 1 g or 1 mL, and absence of Salmonella species in 10 g or 10 mL.".to_string(),
    });

    // ========================================================================
    // Pharmaceutical Quality Attributes
    // ========================================================================

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "Pharmaceutical Quality Systems".to_string(),
        regulation_section: "General".to_string(),
        section_title: "Assay and Content Uniformity".to_string(),
        content: "The assay measures the amount of active pharmaceutical ingredient in the drug product and should be specific for the API, indicating the purity when necessary. Assay procedures should be stability-indicating. The assay acceptance criteria are typically set at 95.0% to 105.0% of label claim for most immediate-release solid oral dosage forms. Content uniformity testing ensures that each dosage unit contains the API within a narrow range around the label claim, typically requiring that individual units fall within 85.0% to 115.0% of label claim, with an acceptance value meeting USP <905> criteria.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "Pharmaceutical Quality Systems".to_string(),
        regulation_section: "General".to_string(),
        section_title: "Related Substances and Degradation Products".to_string(),
        content: "Testing for related substances (impurities) is essential to ensure product quality and patient safety. Impurities can arise from synthesis, degradation during storage, or interaction with excipients or container closure systems. Analytical methods should be capable of detecting and quantifying individual impurities at 0.05% or above. Identification thresholds and qualification thresholds are based on ICH Q3A and Q3B guidelines. For oral dosage forms, the typical reporting threshold is 0.05%, identification threshold is 0.10%, and qualification threshold is 0.15% or 1.0 mg/day intake (whichever is lower).".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "CoA".to_string(),
        regulation_source: "Pharmaceutical Quality Systems".to_string(),
        regulation_section: "General".to_string(),
        section_title: "Physical and Chemical Attributes".to_string(),
        content: "Physical and chemical characteristics of drug products include appearance, color, odor, pH (for liquid formulations), specific gravity or density, viscosity (for semi-solids and liquids), particle size distribution (for suspensions), polymorphic form (where applicable), moisture content (Karl Fischer), hardness and friability (for tablets), disintegration time (for tablets and capsules), and extractable volume (for parenteral products). These attributes should be controlled within specified limits to ensure consistent product quality, performance, and patient acceptability.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GMP".to_string(),
        regulation_source: "Pharmaceutical Quality Systems".to_string(),
        regulation_section: "General".to_string(),
        section_title: "Equipment Qualification and Calibration".to_string(),
        content: "All equipment used in the manufacture, testing, and storage of drug products should be qualified and calibrated according to written procedures. Equipment qualification includes Design Qualification (DQ), Installation Qualification (IQ), Operational Qualification (OQ), and Performance Qualification (PQ). Calibration of instruments should be performed at established intervals using certified reference standards traceable to national or international standards. Records of all qualification and calibration activities should be maintained. Equipment should be suitable for its intended use and should not present any hazard to the products.".to_string(),
    });

    entries.push(RegulatoryEntry {
        document_type: "GDP".to_string(),
        regulation_source: "Pharmaceutical Quality Systems".to_string(),
        regulation_section: "General".to_string(),
        section_title: "Cold Chain Management".to_string(),
        content: "Temperature-sensitive pharmaceutical products require special handling throughout the supply chain. Cold chain management includes: validated packaging systems with temperature monitoring devices, qualified refrigerated vehicles and storage areas (2-8°C), continuous temperature monitoring and recording, established procedures for temperature excursions, trained personnel in cold chain handling, regular calibration of monitoring equipment, and contingency plans for equipment failure. Documentation should demonstrate that products have been maintained within specified temperature ranges throughout storage and distribution.".to_string(),
    });

    tracing::info!("Loaded {} regulatory entries spanning FDA, EU, ICH, and USP guidelines", entries.len());
    entries
}
