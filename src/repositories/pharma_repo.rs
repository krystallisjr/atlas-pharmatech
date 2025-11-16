use sqlx::{PgPool, query, Row};
use uuid::Uuid;
use crate::models::pharmaceutical::{Pharmaceutical, CreatePharmaceuticalRequest, SearchPharmaceuticalRequest};
use crate::middleware::error_handling::Result;

pub struct PharmaceuticalRepository {
    pool: PgPool,
}

impl PharmaceuticalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, request: &CreatePharmaceuticalRequest) -> Result<Pharmaceutical> {
        let row = query(
            r#"
            INSERT INTO pharmaceuticals (brand_name, generic_name, ndc_code, manufacturer, category, description, strength, dosage_form, storage_requirements)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, brand_name, generic_name, ndc_code, manufacturer, category, description, strength, dosage_form, storage_requirements, created_at
            "#
        )
        .bind(&request.brand_name)
        .bind(&request.generic_name)
        .bind(&request.ndc_code)
        .bind(&request.manufacturer)
        .bind(&request.category)
        .bind(&request.description)
        .bind(&request.strength)
        .bind(&request.dosage_form)
        .bind(&request.storage_requirements)
        .fetch_one(&self.pool)
        .await?;

        Ok(Pharmaceutical {
            id: row.try_get("id")?,
            brand_name: row.try_get("brand_name")?,
            generic_name: row.try_get("generic_name")?,
            ndc_code: row.try_get("ndc_code")?,
            manufacturer: row.try_get("manufacturer")?,
            category: row.try_get("category")?,
            description: row.try_get("description")?,
            strength: row.try_get("strength")?,
            dosage_form: row.try_get("dosage_form")?,
            storage_requirements: row.try_get("storage_requirements")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Pharmaceutical>> {
        let row = query(
            "SELECT id, brand_name, generic_name, ndc_code, manufacturer, category, description, strength, dosage_form, storage_requirements, created_at FROM pharmaceuticals WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Pharmaceutical {
                id: row.try_get("id")?,
                brand_name: row.try_get("brand_name")?,
                generic_name: row.try_get("generic_name")?,
                ndc_code: row.try_get("ndc_code")?,
                manufacturer: row.try_get("manufacturer")?,
                category: row.try_get("category")?,
                description: row.try_get("description")?,
                strength: row.try_get("strength")?,
                dosage_form: row.try_get("dosage_form")?,
                storage_requirements: row.try_get("storage_requirements")?,
                created_at: row.try_get("created_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn find_by_ndc(&self, ndc_code: &str) -> Result<Option<Pharmaceutical>> {
        let row = query(
            "SELECT id, brand_name, generic_name, ndc_code, manufacturer, category, description, strength, dosage_form, storage_requirements, created_at FROM pharmaceuticals WHERE ndc_code = $1"
        )
        .bind(ndc_code)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Pharmaceutical {
                id: row.try_get("id")?,
                brand_name: row.try_get("brand_name")?,
                generic_name: row.try_get("generic_name")?,
                ndc_code: row.try_get("ndc_code")?,
                manufacturer: row.try_get("manufacturer")?,
                category: row.try_get("category")?,
                description: row.try_get("description")?,
                strength: row.try_get("strength")?,
                dosage_form: row.try_get("dosage_form")?,
                storage_requirements: row.try_get("storage_requirements")?,
                created_at: row.try_get("created_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn search(&self, request: &SearchPharmaceuticalRequest) -> Result<Vec<Pharmaceutical>> {
        let limit = request.limit.unwrap_or(50).min(100);
        let offset = request.offset.unwrap_or(0);

        let mut query_str = "SELECT id, brand_name, generic_name, ndc_code, manufacturer, category, description, strength, dosage_form, storage_requirements, created_at FROM pharmaceuticals WHERE 1=1".to_string();
        let mut param_count = 1;

        if let Some(ref query_str_param) = request.query {
            query_str.push_str(&format!(" AND (brand_name ILIKE ${} OR generic_name ILIKE ${} OR manufacturer ILIKE ${})", param_count, param_count + 1, param_count + 2));
            param_count += 3;
        }

        if let Some(ref brand_name) = request.brand_name {
            query_str.push_str(&format!(" AND brand_name ILIKE ${}", param_count));
            param_count += 1;
        }

        if let Some(ref generic_name) = request.generic_name {
            query_str.push_str(&format!(" AND generic_name ILIKE ${}", param_count));
            param_count += 1;
        }

        if let Some(ref manufacturer) = request.manufacturer {
            query_str.push_str(&format!(" AND manufacturer ILIKE ${}", param_count));
            param_count += 1;
        }

        if let Some(ref category) = request.category {
            query_str.push_str(&format!(" AND category = ${}", param_count));
            param_count += 1;
        }

        if let Some(ref ndc_code) = request.ndc_code {
            query_str.push_str(&format!(" AND ndc_code = ${}", param_count));
            param_count += 1;
        }

        query_str.push_str(" ORDER BY brand_name ASC");
        query_str.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut query_builder = query(&query_str);

        let pattern = if let Some(ref query_str_param) = request.query {
            Some(format!("%{}%", query_str_param))
        } else {
            None
        };
        
        if let Some(ref pattern) = pattern {
            query_builder = query_builder.bind(pattern).bind(pattern).bind(pattern);
        }

        if let Some(ref brand_name) = request.brand_name {
            query_builder = query_builder.bind(format!("%{}%", brand_name));
        }

        if let Some(ref generic_name) = request.generic_name {
            query_builder = query_builder.bind(format!("%{}%", generic_name));
        }

        if let Some(ref manufacturer) = request.manufacturer {
            query_builder = query_builder.bind(format!("%{}%", manufacturer));
        }

        if let Some(ref category) = request.category {
            query_builder = query_builder.bind(category);
        }

        if let Some(ref ndc_code) = request.ndc_code {
            query_builder = query_builder.bind(ndc_code);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await?;

        let mut pharmaceuticals = Vec::new();
        for row in rows {
            pharmaceuticals.push(Pharmaceutical {
                id: row.try_get("id")?,
                brand_name: row.try_get("brand_name")?,
                generic_name: row.try_get("generic_name")?,
                ndc_code: row.try_get("ndc_code")?,
                manufacturer: row.try_get("manufacturer")?,
                category: row.try_get("category")?,
                description: row.try_get("description")?,
                strength: row.try_get("strength")?,
                dosage_form: row.try_get("dosage_form")?,
                storage_requirements: row.try_get("storage_requirements")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(pharmaceuticals)
    }

    pub async fn ndc_exists(&self, ndc_code: &str) -> Result<bool> {
        let row = query("SELECT EXISTS(SELECT 1 FROM pharmaceuticals WHERE ndc_code = $1) as exists")
            .bind(ndc_code)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get::<bool, _>("exists").unwrap_or(false))
    }

    pub async fn get_manufacturers(&self) -> Result<Vec<String>> {
        let rows = query("SELECT DISTINCT manufacturer FROM pharmaceuticals ORDER BY manufacturer")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter()
            .map(|row| row.try_get::<String, _>("manufacturer"))
            .collect::<std::result::Result<Vec<String>, _>>()?)
    }

    pub async fn get_categories(&self) -> Result<Vec<String>> {
        let rows = query("SELECT DISTINCT category FROM pharmaceuticals WHERE category IS NOT NULL ORDER BY category")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter()
            .filter_map(|row| row.try_get::<Option<String>, _>("category").ok().flatten())
            .collect())
    }
}