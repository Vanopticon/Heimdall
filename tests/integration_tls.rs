use rcgen::generate_simple_self_signed;

#[tokio::test]
async fn integration_tls_rejects_self_signed_cert() -> Result<(), Box<dyn std::error::Error>> {
	// Generate a self-signed cert for `localhost` and write to temporary files
	let cert = generate_simple_self_signed(vec!["localhost".into()])?;
	let cert_pem = cert.serialize_pem()?;
	let key_pem = cert.serialize_private_key_pem();

	let tmpdir = tempfile::tempdir()?;
	let cert_path = tmpdir.path().join("cert.pem");
	let key_path = tmpdir.path().join("key.pem");
	std::fs::write(&cert_path, cert_pem.as_bytes())?;
	std::fs::write(&key_path, key_pem.as_bytes())?;

	// Load server-side certs and key using the repository helpers
	let certs = vanopticon_heimdall::tls_utils::load_certs(&cert_path)?;
	let key = vanopticon_heimdall::tls_utils::load_private_key(&key_path)?;

	// Expect building a server config for a self-signed cert to fail
	let res = vanopticon_heimdall::tls_utils::build_server_config_tls13(certs, key);
	assert!(
		res.is_err(),
		"self-signed certificate should be rejected by server config builder"
	);

	Ok(())
}
