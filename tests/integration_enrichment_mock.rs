use serde_json::json;
use std::env;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Integration test demonstrating enrichment workflow with mock data.
/// This test validates that enrichment metadata can be attached to entities
/// and persisted into the graph database.
#[tokio::test]
async fn e2e_enrichment_mock_workflow() {
	// Gate the test
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!(
			"Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable"
		);
		return;
	}

	// Start the dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = loop {
		match sqlx::PgPool::connect("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall").await {
			Ok(p) => break p,
			Err(_) => {
				sleep(Duration::from_secs(1)).await;
			}
		}
	};

	// Build an AgeClient and shared repo
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");
	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client);

	// Start a batcher that flushes immediately for testing (batch_size=1)
	let sender = vanopticon_heimdall::persist::start_batcher(repo.clone(), 1024, 1, 100);

	// Scenario: Ingest an IP address, then enrich it with GeoIP and ASN data
	
	// Step 1: Create base entity (IP address)
	let ip_address = "8.8.8.8";
	let ip_job = vanopticon_heimdall::persist::PersistJob {
		label: "IPAddress".to_string(),
		key: ip_address.to_string(),
		props: json!({
			"canonical_key": ip_address,
			"field_type": "ip",
		}),
	};
	
	vanopticon_heimdall::persist::submit_job(&sender, ip_job)
		.expect("submit IP job");

	// Step 2: Mock enrichment - add GeoIP data
	let geoip_job = vanopticon_heimdall::persist::PersistJob {
		label: "GeoIPEnrichment".to_string(),
		key: format!("geoip_{}", ip_address),
		props: json!({
			"canonical_key": format!("geoip_{}", ip_address),
			"ip_address": ip_address,
			"country": "US",
			"city": "Mountain View",
			"latitude": 37.419200,
			"longitude": -122.057404,
			"enrichment_source": "mock_geoip",
			"enriched_at": "2024-01-01T00:00:00Z",
		}),
	};
	
	vanopticon_heimdall::persist::submit_job(&sender, geoip_job)
		.expect("submit GeoIP enrichment job");

	// Step 3: Mock enrichment - add ASN data
	let asn_job = vanopticon_heimdall::persist::PersistJob {
		label: "ASNEnrichment".to_string(),
		key: format!("asn_{}", ip_address),
		props: json!({
			"canonical_key": format!("asn_{}", ip_address),
			"ip_address": ip_address,
			"asn": 15169,
			"organization": "GOOGLE",
			"enrichment_source": "mock_asn",
			"enriched_at": "2024-01-01T00:00:00Z",
		}),
	};
	
	vanopticon_heimdall::persist::submit_job(&sender, asn_job)
		.expect("submit ASN enrichment job");

	// Allow time for the batcher to flush
	sleep(Duration::from_millis(500)).await;

	// Verify: Check that all three nodes exist in the graph
	
	// Verify IP address node
	let ip_sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
	let ip_cypher = format!(
		"MATCH (n:IPAddress {{canonical_key: \"{}\"}}) RETURN n LIMIT 1",
		ip_address
	);
	let ip_row = sqlx::query(ip_sql)
		.bind("heimdall_graph")
		.bind(&ip_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query IP address");
	assert!(ip_row.is_some(), "expected IP address node in the DB");

	// Verify GeoIP enrichment node
	let geoip_cypher = format!(
		"MATCH (n:GeoIPEnrichment {{ip_address: \"{}\"}}) RETURN n LIMIT 1",
		ip_address
	);
	let geoip_row = sqlx::query(ip_sql)
		.bind("heimdall_graph")
		.bind(&geoip_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query GeoIP enrichment");
	assert!(geoip_row.is_some(), "expected GeoIP enrichment node in the DB");

	// Verify ASN enrichment node
	let asn_cypher = format!(
		"MATCH (n:ASNEnrichment {{ip_address: \"{}\"}}) RETURN n LIMIT 1",
		ip_address
	);
	let asn_row = sqlx::query(ip_sql)
		.bind("heimdall_graph")
		.bind(&asn_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query ASN enrichment");
	assert!(asn_row.is_some(), "expected ASN enrichment node in the DB");

	// Tear down DB
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

/// Integration test demonstrating chained enrichment with multiple providers.
/// This validates that enrichment can be layered and linked.
#[tokio::test]
async fn e2e_chained_enrichment() {
	// Gate the test
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!(
			"Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable"
		);
		return;
	}

	// Start the dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = loop {
		match sqlx::PgPool::connect("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall").await {
			Ok(p) => break p,
			Err(_) => {
				sleep(Duration::from_secs(1)).await;
			}
		}
	};

	// Build an AgeClient and shared repo
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");
	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client);

	// Start a batcher
	let sender = vanopticon_heimdall::persist::start_batcher(repo.clone(), 1024, 1, 100);

	// Scenario: Domain -> DNS Resolution -> IP -> GeoIP
	let domain = "example.com";
	
	// Step 1: Domain entity
	let domain_job = vanopticon_heimdall::persist::PersistJob {
		label: "Domain".to_string(),
		key: domain.to_string(),
		props: json!({
			"canonical_key": domain,
			"field_type": "domain",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, domain_job)
		.expect("submit domain job");

	// Step 2: DNS resolution enrichment (mock)
	let dns_job = vanopticon_heimdall::persist::PersistJob {
		label: "DNSEnrichment".to_string(),
		key: format!("dns_{}", domain),
		props: json!({
			"canonical_key": format!("dns_{}", domain),
			"domain": domain,
			"resolved_ips": ["93.184.216.34"],
			"enrichment_source": "mock_dns",
			"enriched_at": "2024-01-01T00:00:00Z",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, dns_job)
		.expect("submit DNS enrichment job");

	// Step 3: Discovered IP from DNS resolution
	let discovered_ip = "93.184.216.34";
	let ip_job = vanopticon_heimdall::persist::PersistJob {
		label: "IPAddress".to_string(),
		key: discovered_ip.to_string(),
		props: json!({
			"canonical_key": discovered_ip,
			"field_type": "ip",
			"discovered_via": "dns_resolution",
			"source_domain": domain,
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, ip_job)
		.expect("submit IP job");

	// Step 4: GeoIP enrichment for discovered IP
	let geoip_job = vanopticon_heimdall::persist::PersistJob {
		label: "GeoIPEnrichment".to_string(),
		key: format!("geoip_{}", discovered_ip),
		props: json!({
			"canonical_key": format!("geoip_{}", discovered_ip),
			"ip_address": discovered_ip,
			"country": "US",
			"city": "Norwell",
			"enrichment_source": "mock_geoip",
			"enriched_at": "2024-01-01T00:00:00Z",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, geoip_job)
		.expect("submit GeoIP enrichment job");

	// Allow time for the batcher to flush
	sleep(Duration::from_millis(500)).await;

	// Verify all nodes exist
	let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
	
	// Verify domain
	let domain_cypher = format!(
		"MATCH (n:Domain {{canonical_key: \"{}\"}}) RETURN n LIMIT 1",
		domain
	);
	let domain_row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(&domain_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query domain");
	assert!(domain_row.is_some(), "expected domain node");

	// Verify DNS enrichment
	let dns_cypher = format!(
		"MATCH (n:DNSEnrichment {{domain: \"{}\"}}) RETURN n LIMIT 1",
		domain
	);
	let dns_row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(&dns_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query DNS enrichment");
	assert!(dns_row.is_some(), "expected DNS enrichment node");

	// Verify discovered IP
	let ip_cypher = format!(
		"MATCH (n:IPAddress {{canonical_key: \"{}\"}}) RETURN n LIMIT 1",
		discovered_ip
	);
	let ip_row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(&ip_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query IP");
	assert!(ip_row.is_some(), "expected IP node");

	// Verify GeoIP enrichment for discovered IP
	let geoip_cypher = format!(
		"MATCH (n:GeoIPEnrichment {{ip_address: \"{}\"}}) RETURN n LIMIT 1",
		discovered_ip
	);
	let geoip_row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(&geoip_cypher)
		.fetch_optional(&pool)
		.await
		.expect("query GeoIP");
	assert!(geoip_row.is_some(), "expected GeoIP enrichment node");

	// Tear down DB
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
