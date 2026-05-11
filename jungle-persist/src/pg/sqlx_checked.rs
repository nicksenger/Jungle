//! Compile-only SQLx checked-query coverage for Postgres statements.
//!
//! This module is gated behind the `sqlx-checked` feature and is not called at runtime.
//! Its purpose is to force `sqlx::query!` macro expansion against this crate's SQL.

use uuid::Uuid;

#[allow(dead_code)]
pub(crate) fn compile_checked_queries() {
    let namespace = "default";
    let animal_id: i32 = 1;
    let generation: i32 = 0;
    let journey_id = Uuid::nil();
    let work_item_id = Uuid::nil();
    let owner_id = Uuid::nil();
    let timer_id = Uuid::nil();
    let status: i16 = 0;
    let seed: Vec<u8> = Vec::new();
    let payload: Vec<u8> = Vec::new();
    let supported_ids: Vec<i32> = vec![animal_id];
    let supported_generations: Vec<i32> = vec![generation];
    let wake_at_unix_ms: i64 = 0;
    let lease_ttl_ms: i64 = 0;
    let sequence_id: i64 = 0;

    let _ = sqlx::query!(
        r#"
        SELECT generation
        FROM animal_generations
        WHERE namespace = $1 AND animal_id = $2
        "#,
        namespace,
        animal_id
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO journeys (id, namespace, animal_id, generation, status, seed)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        journey_id,
        namespace,
        animal_id,
        generation,
        status,
        seed
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO work_items (id, journey_id, kind, status, expiry)
        VALUES ($1, $2, $3, $4, NOW())
        "#,
        work_item_id,
        journey_id,
        0_i16,
        status
    );

    let _ = sqlx::query!(
        r#"
        SELECT kind, data
        FROM events
        WHERE journey_id = $1
        ORDER BY sequence_id
        "#,
        journey_id
    );

    let _ = sqlx::query!(
        r#"
        SELECT status
        FROM journeys
        WHERE id = $1
        "#,
        journey_id
    );

    let _ = sqlx::query!(
        r#"
        SELECT data
        FROM animal_appearances
        WHERE journey_id = $1
        "#,
        journey_id
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO animal_appearances (journey_id, data, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (journey_id)
        DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()
        "#,
        journey_id,
        payload
    );

    let _ = sqlx::query!(
        r#"
        WITH next_sequence AS (
            SELECT COALESCE(MAX(sequence_id) + 1, 0) AS sequence_id
            FROM animal_perturbations
            WHERE journey_id = $1
        )
        INSERT INTO animal_perturbations (
            journey_id,
            sequence_id,
            data,
            status,
            claimed_at,
            lease_until
        )
        SELECT $1, next_sequence.sequence_id, $2, $3, NULL, NULL
        FROM next_sequence
        "#,
        journey_id,
        payload,
        0_i16
    );

    let _ = sqlx::query!(
        r#"
        WITH next_item AS (
            SELECT journey_id, sequence_id
            FROM animal_perturbations
            WHERE journey_id = $1
              AND (status = $2 OR (status = $3 AND lease_until < NOW()))
            ORDER BY sequence_id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        ),
        claimed AS (
            UPDATE animal_perturbations ap
            SET status = $3,
                claimed_at = NOW(),
                lease_until = NOW() + INTERVAL '30 seconds'
            FROM next_item ni
            WHERE ap.journey_id = ni.journey_id
              AND ap.sequence_id = ni.sequence_id
            RETURNING ap.sequence_id, ap.data
        )
        SELECT sequence_id, data
        FROM claimed
        "#,
        journey_id,
        0_i16,
        1_i16
    );

    let _ = sqlx::query!(
        r#"
        DELETE FROM animal_perturbations
        WHERE journey_id = $1 AND sequence_id = $2
        "#,
        journey_id,
        sequence_id
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO journey_leases (journey_id, owner_id, lease_until, heartbeat_at)
        VALUES ($1, $2, NOW() + ($3::BIGINT * INTERVAL '1 millisecond'), NOW())
        ON CONFLICT (journey_id)
        DO UPDATE SET owner_id = EXCLUDED.owner_id,
                      lease_until = EXCLUDED.lease_until,
                      heartbeat_at = EXCLUDED.heartbeat_at
        "#,
        journey_id,
        owner_id,
        lease_ttl_ms
    );

    let _ = sqlx::query!(
        r#"
        WITH next_wake AS (
            SELECT id
            FROM owner_wakes
            WHERE owner_id = $1
            ORDER BY created_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        DELETE FROM owner_wakes ow
        USING next_wake nw
        WHERE ow.id = nw.id
        RETURNING ow.journey_id, ow.timer_id
        "#,
        owner_id
    );

    let _ = sqlx::query!(
        r#"
        UPDATE journeys
        SET status = $2
        WHERE id = $1
        "#,
        journey_id,
        3_i16
    );

    let _ = sqlx::query!(
        r#"
        UPDATE journeys
        SET status = $2
        WHERE id = $1 AND status = $3
        "#,
        journey_id,
        1_i16,
        0_i16
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO animal_generations (namespace, animal_id, generation, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (namespace, animal_id)
        DO UPDATE SET
            generation = EXCLUDED.generation,
            updated_at = NOW()
        WHERE animal_generations.generation < EXCLUDED.generation
        "#,
        namespace,
        animal_id,
        generation
    );

    let _ = sqlx::query!(
        r#"
        WITH supported AS (
            SELECT * FROM UNNEST($2::INT4[], $3::INT4[]) AS s(animal_id, generation)
        ),
        candidate AS (
            SELECT wi.id
            FROM work_items wi
            INNER JOIN journeys j ON j.id = wi.journey_id
            INNER JOIN supported s
                ON s.animal_id = j.animal_id
               AND s.generation = j.generation
            WHERE j.namespace = $1
              AND (wi.status = $4 OR (wi.status = $5 AND wi.expiry < NOW()))
            ORDER BY wi.expiry, wi.id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        ),
        claimed AS (
            UPDATE work_items wi
            SET status = $5,
                expiry = NOW() + INTERVAL '30 seconds'
            FROM candidate c
            WHERE wi.id = c.id
            RETURNING wi.journey_id, wi.kind
        )
        SELECT c.journey_id, c.kind, f.animal_id, f.generation, f.seed
        FROM claimed c
        INNER JOIN journeys f ON f.id = c.journey_id
        "#,
        namespace,
        &supported_ids,
        &supported_generations,
        0_i16,
        1_i16
    );

    let _ = sqlx::query!(
        r#"
        WITH next_sequence AS (
            SELECT COALESCE(MAX(sequence_id) + 1, 0) AS sequence_id
            FROM events
            WHERE journey_id = $1
        )
        INSERT INTO events (journey_id, sequence_id, kind, data)
        SELECT $1, next_sequence.sequence_id, $2, $3
        FROM next_sequence
        "#,
        journey_id,
        4_i16,
        payload
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO timer_tasks (id, journey_id, status, visible_at, fired_at)
        VALUES ($1, $2, $3, to_timestamp($4::BIGINT::double precision / 1000.0), NULL)
        ON CONFLICT (id) DO NOTHING
        "#,
        timer_id,
        journey_id,
        0_i16,
        wake_at_unix_ms
    );

    let _ = sqlx::query!(
        r#"
        SELECT id, journey_id
        FROM timer_tasks
        WHERE status = $1 AND visible_at <= NOW()
        ORDER BY visible_at, id
        FOR UPDATE SKIP LOCKED
        LIMIT 1
        "#,
        0_i16
    );

    let _ = sqlx::query!(
        r#"
        UPDATE timer_tasks
        SET status = $2, fired_at = NOW()
        WHERE id = $1 AND status = $3
        "#,
        timer_id,
        1_i16,
        0_i16
    );

    let _ = sqlx::query!(
        r#"
        SELECT owner_id
        FROM journey_leases
        WHERE journey_id = $1 AND lease_until > NOW()
        LIMIT 1
        "#,
        journey_id
    );

    let _ = sqlx::query!(
        r#"
        INSERT INTO owner_wakes (id, owner_id, journey_id, timer_id, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        "#,
        Uuid::new_v4(),
        owner_id,
        journey_id,
        timer_id
    );
}
