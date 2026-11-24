Value::Object(HashMap::from([
    ("icon".into(), Value::reference("https://example.com/cat.png").unwrap()),
    ("mail".into(), Value::reference("mailto:kris@example.com").unwrap()),
    ("peer".into(), Value::reference("ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046").unwrap()),
    ("spec".into(), Value::reference("urn:ietf:rfc:3986").unwrap()),
]))
