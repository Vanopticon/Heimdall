import { PgConnectionManager, QueryExecutor, SQLGenerator, VertexOperations, EdgeOperations } from 'age-schema-client';

// Lightweight wrapper around age-schema-client for Heimdall
// Initializes a client using environment variables and registers
// a minimal schema derived from sql/v1/*.sql. Exports simple accessors.

type Config = {
	host?: string;
	port?: number;
	database?: string;
	user?: string;
	password?: string;
	graph?: string;
};

export const DEFAULT_CONFIG: Config = {
	host: process.env['PGHOST'] || '127.0.0.1',
	port: process.env['PGPORT'] ? parseInt(process.env['PGPORT'] as string) : 5432,
	database: process.env['PGDATABASE'] || 'postgres',
	user: process.env['PGUSER'] || 'postgres',
	password: process.env['PGPASSWORD'] || 'postgres',
	graph: process.env['AGE_GRAPH'] || 'dumps_graph',
};

// Minimal schema inferred from `sql/v1/001-create_graph.sql`
// We define the vertex labels and a couple of edge types used by that SQL.
export const HEIMDALL_SCHEMA = {
	version: '1.0.0',
	vertices: {
		sources: {
			properties: {
				name: { type: 'string', required: true },
			},
		},
		dumps: {
			properties: {
				name: { type: 'string', required: true },
			},
		},
		fields: {
			properties: {
				name: { type: 'string', required: true },
			},
		},
		field_data: {
			properties: {
				name: { type: 'string' },
			},
		},
		rows: {
			properties: {
				row_key: { type: 'string', required: true },
			},
		},
		field_value: {
			properties: {
				value: { type: 'string', required: true },
				field: { type: 'string' },
				row_key: { type: 'string' },
			},
		},
		NPI_Category: {
			properties: {
				name: { type: 'string', required: true },
			},
		},
		sightings: {
			properties: {
				name: { type: 'string' },
			},
		},
		ip: {
			properties: {
				address: { type: 'string', required: true },
			},
		},
	},
	edges: {
		HAS_SUBCATEGORY: {
			properties: {},
			from: ['NPI_Category'],
			to: ['NPI_Category'],
		},
		HAS_MEMBER: {
			properties: {},
			from: ['NPI_Category'],
			to: ['ip'],
		},
		ROUTES_TO: {
			properties: {},
			from: ['ip'],
			to: ['ip'],
		},
		ROOT_CONTAINS: {
			properties: {},
			from: ['dumps', 'fields', 'field_data', 'sources', 'sightings', 'NPI_Category'],
			to: ['dumps', 'fields', 'field_data', 'sources', 'sightings', 'NPI_Category'],
		},
		DUMP_HAS_FIELD: {
			properties: {},
			from: ['dumps'],
			to: ['fields'],
		},
		FIELD_HAS_DATA: {
			properties: {},
			from: ['fields'],
			to: ['field_data'],
		},
		FIELD_LINK: {
			properties: {},
			from: ['fields'],
			to: ['fields'],
		},
		FIELD_HAS_NPI: {
			properties: {},
			from: ['fields'],
			to: ['NPI_Category'],
		},
		FIELD_VALUE_HAS_NPI: {
			properties: {},
			from: ['field_value'],
			to: ['NPI_Category'],
		},
		DUMP_HAS_ROW: {
			properties: {},
			from: ['dumps'],
			to: ['rows'],
		},
		ROW_HAS_FIELD_VALUE: {
			properties: {},
			from: ['rows'],
			to: ['field_value'],
		},
		FIELD_VALUE_FOR_FIELD: {
			properties: {},
			from: ['field_value'],
			to: ['fields'],
		},
		SIGHTING_OF: {
			properties: {},
			from: ['sightings'],
			to: ['field_value'],
		},
	},
};

type InternalClient = {
	connectionManager: any;
	connection: any;
	queryExecutor: any;
	sqlGenerator: any;
	vertexOperations: any;
	edgeOperations?: any;
	graph: string;
};

let client: InternalClient | null = null;

// Prefer HMD_DATABASE_URL; fall back to other common env names.
const CONNECTION_URL =
	process.env['HMD_DATABASE_URL'] || process.env['DATABASE_URL'] || process.env['PG_CONNECTION_STRING'] || '';

export async function initAgeClient(cfg?: Partial<Config>) {
	const config = { ...DEFAULT_CONFIG, ...(cfg || {}) };

	// Build PgConnectionConfig either from HMD_DATABASE_URL or explicit parts
	let connConfig: any = null;
	if (CONNECTION_URL) {
		try {
			const url = new URL(CONNECTION_URL);
			connConfig = {
				host: url.hostname,
				port: url.port ? parseInt(url.port) : 5432,
				database: url.pathname ? url.pathname.replace(/^\//, '') : config.database,
				user: url.username || config.user,
				password: url.password || config.password,
			};
		} catch (err) {
			// if parsing fails, fall back to defaults
			connConfig = {
				host: config.host,
				port: config.port,
				database: config.database,
				user: config.user,
				password: config.password,
			};
		}
	} else {
		connConfig = {
			host: config.host,
			port: config.port,
			database: config.database,
			user: config.user,
			password: config.password,
		};
	}

	const connectionManager = new PgConnectionManager(connConfig);
	const connection = await connectionManager.getConnection();
	const queryExecutor = new QueryExecutor(connection);
	const sqlGenerator = new SQLGenerator(HEIMDALL_SCHEMA as any);
	const vertexOperations = new VertexOperations(HEIMDALL_SCHEMA as any, queryExecutor, sqlGenerator);
	const edgeOperations = new EdgeOperations(HEIMDALL_SCHEMA as any, queryExecutor, sqlGenerator, vertexOperations);

	client = {
		connectionManager,
		connection,
		queryExecutor,
		sqlGenerator,
		vertexOperations,
		edgeOperations,
		graph: config.graph || 'dumps_graph',
	};

	return client;
}

export function getAgeClient() {
	if (!client) throw new Error('AGE client not initialized; call initAgeClient() first');
	return client;
}

function propsToCypherMap(props: Record<string, unknown>) {
	const parts: string[] = [];
	for (const [k, v] of Object.entries(props)) {
		if (v === null || v === undefined) continue;
		if (typeof v === 'string') {
			// simple string escaping
			parts.push(`${k}: '${String(v).replace(/'/g, "\\'")}'`);
		} else if (typeof v === 'number' || typeof v === 'boolean') {
			parts.push(`${k}: ${v}`);
		} else {
			parts.push(`${k}: ${JSON.stringify(v)}`);
		}
	}
	return `{ ${parts.join(', ')} }`;
}

// Basic accessors using AGE / Cypher via the AgeSchemaClient. The client
// exposes higher-level APIs, but to be defensive we fall back to executing
// raw cypher SQL through the client when needed.

export async function createVertex(label: string, props: Record<string, unknown>) {
	const c = getAgeClient();
	// Prefer the high-level vertexOperations API
	if (c.vertexOperations && typeof c.vertexOperations.createVertex === 'function') {
		return await c.vertexOperations.createVertex(label, props as any);
	}

	// Fallback: execute a cypher query via the raw connection
	const map = propsToCypherMap(props);
	const cypher = `CREATE (n:${label} ${map}) RETURN n`;
	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (n agtype);`;
	return await c.connection.query(sql);
}

export async function findAll(label: string) {
	const c = getAgeClient();
	if (c.vertexOperations && typeof c.vertexOperations.findAll === 'function') {
		return await c.vertexOperations.findAll(label as any);
	}
	const cypher = `MATCH (n:${label}) RETURN n`;
	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (n agtype);`;
	return await c.connection.query(sql);
}

// Create an `ip` vertex and link it from the `NPI_Category {name: 'INFRASTRUCTURE'}` root.
export async function createIp(address: string, props: Record<string, unknown> = {}) {
	const c = getAgeClient();

	// Build a map of extra properties (excluding address)
	const extras: Record<string, unknown> = { ...props };
	delete (extras as any).address;

	const extrasMap = Object.keys(extras).length ? propsToCypherMap(extras) : '{}';
	const safeAddress = String(address).replace(/'/g, "\\'");

	const cypher = `
		MERGE (n:NPI_Category {name: 'INFRASTRUCTURE'})
		MERGE (i:ip {address: '${safeAddress}'})
		SET i += ${extrasMap}
		MERGE (n)-[:HAS_MEMBER]->(i)
		RETURN n, i
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (n agtype, i agtype);`;
	return await c.connection.query(sql);
}

// Create a `ROUTES_TO` relationship from one IP to another (idempotent).
export async function createRoute(fromAddress: string, toAddress: string) {
	const c = getAgeClient();
	const a = String(fromAddress).replace(/'/g, "\\'");
	const b = String(toAddress).replace(/'/g, "\\'");

	const cypher = `
		MERGE (a:ip {address: '${a}'})
		MERGE (b:ip {address: '${b}'})
		MERGE (a)-[:ROUTES_TO]->(b)
		RETURN a, b
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (a agtype, b agtype);`;
	return await c.connection.query(sql);
}

// Create or ensure a `dumps` vertex and link it from the `dumps {name: 'ROOT'}` root.
export async function createDump(name: string, props: Record<string, unknown> = {}) {
	const c = getAgeClient();
	const safeName = String(name).replace(/'/g, "\\'");
	const map = Object.keys(props).length ? propsToCypherMap(props) : '{}';

	const cypher = `
		MERGE (r:dumps {name: 'ROOT'})
		MERGE (d:dumps {name: '${safeName}'})
		SET d += ${map}
		MERGE (r)-[:ROOT_CONTAINS]->(d)
		RETURN r, d
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (r agtype, d agtype);`;
	return await c.connection.query(sql);
}

// Create or ensure a `fields` vertex, link it to the parent `dumps` vertex,
// and attach it to the `fields {name: 'ROOT'}` root.
export async function createField(dumpName: string, fieldName: string, props: Record<string, unknown> = {}) {
	const c = getAgeClient();
	const safeDump = String(dumpName).replace(/'/g, "\\'");
	const safeField = String(fieldName).replace(/'/g, "\\'");
	const map = Object.keys(props).length ? propsToCypherMap(props) : '{}';

	const cypher = `
		MERGE (rd: dumps {name: 'ROOT'})
		MERGE (rf: fields {name: 'ROOT'})
		MERGE (d:dumps {name: '${safeDump}'})
		MERGE (f:fields {name: '${safeField}'})
		SET f += ${map}
		MERGE (rd)-[:ROOT_CONTAINS]->(d)
		MERGE (d)-[:DUMP_HAS_FIELD]->(f)
		MERGE (rf)-[:ROOT_CONTAINS]->(f)
		RETURN d, f
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (d agtype, f agtype);`;
	return await c.connection.query(sql);
}

// Create a field data row/value for a particular field (identified by name)
// `rowKey` may be any identifier (number or string) to group values by row.
export async function createFieldValue(
	dumpName: string,
	fieldName: string,
	rowKey: string | number,
	value: unknown,
	props: Record<string, unknown> = {},
) {
	const c = getAgeClient();
	const safeDump = String(dumpName).replace(/'/g, "\\'");
	const safeField = String(fieldName).replace(/'/g, "\\'");
	const safeRow = String(rowKey).replace(/'/g, "\\'");
	const safeValueRaw = typeof value === 'string' ? String(value) : JSON.stringify(value);
	const safeValue = String(safeValueRaw).replace(/'/g, "\\'");
	const extrasMap = Object.keys(props).length ? propsToCypherMap(props) : '{}';

	// Check for existing field_value nodes with the same value (across fields).
	// If found, create a sighting linking the existing and the new occurrence.
	const detectedAt = new Date().toISOString();

	const cypher = `
		MERGE (d:dumps {name: '${safeDump}'})
		MERGE (rd:rows {dump: '${safeDump}', row_key: '${safeRow}'})
		MERGE (f:fields {name: '${safeField}'})
		MERGE (fv:field_value {value: '${safeValue}', field: '${safeField}', row_key: '${safeRow}', dump: '${safeDump}'})
		SET fv += ${extrasMap}
		MERGE (d)-[:DUMP_HAS_ROW]->(rd)
		MERGE (rd)-[:ROW_HAS_FIELD_VALUE]->(fv)
		MERGE (fv)-[:FIELD_VALUE_FOR_FIELD]->(f)

		// find other occurrences of same value on different fields
		WITH fv
		MATCH (other:field_value {value: '${safeValue}'})
		WHERE other.field IS NULL OR other.field <> fv.field OR other.dump IS NULL OR other.dump <> fv.dump
		WITH fv, collect(DISTINCT other) as others
		WHERE size(others) > 0
		CREATE (s:sightings {detected_at: '${detectedAt}'})
		FOREACH (o IN others | MERGE (s)-[:SIGHTING_OF]->(o))
		MERGE (s)-[:SIGHTING_OF]->(fv)
		RETURN fv, others
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (fv agtype, others agtype);`;
	return await c.connection.query(sql);
}

export async function linkFieldToNpi(fieldName: string, npiName: string) {
	const c = getAgeClient();
	const f = String(fieldName).replace(/'/g, "\\'");
	const n = String(npiName).replace(/'/g, "\\'");

	const cypher = `
		MERGE (f:fields {name: '${f}'})
		MERGE (n:NPI_Category {name: '${n}'})
		MERGE (f)-[:FIELD_HAS_NPI]->(n)
		RETURN f, n
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (f agtype, n agtype);`;
	return await c.connection.query(sql);
}

export async function linkFieldValueToNpi(fieldName: string, rowKey: string | number, npiName: string) {
	const c = getAgeClient();
	const f = String(fieldName).replace(/'/g, "\\'");
	const r = String(rowKey).replace(/'/g, "\\'");
	const n = String(npiName).replace(/'/g, "\\'");

	const cypher = `
		MERGE (fv:field_value {field: '${f}', row_key: '${r}'})
		MERGE (n:NPI_Category {name: '${n}'})
		MERGE (fv)-[:FIELD_VALUE_HAS_NPI]->(n)
		RETURN fv, n
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (fv agtype, n agtype);`;
	return await c.connection.query(sql);
}

// Link two fields together (for example: IDENTIFICATION ID -> IDENTIFICATION Credential)
export async function linkFields(parentField: string, childField: string) {
	const c = getAgeClient();
	const p = String(parentField).replace(/'/g, "\\'");
	const ch = String(childField).replace(/'/g, "\\'");

	const cypher = `
		MERGE (p:fields {name: '${p}'})
		MERGE (c:fields {name: '${ch}'})
		MERGE (p)-[:FIELD_LINK]->(c)
		RETURN p, c
	`;

	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (p agtype, c agtype);`;
	return await c.connection.query(sql);
}

// Transaction support for atomic operations
export async function withTransaction<T>(fn: (connection: any) => Promise<T>): Promise<T> {
	const c = getAgeClient();
	const conn = await c.connectionManager.getConnection();

	try {
		await conn.query('BEGIN');
		const result = await fn(conn);
		await conn.query('COMMIT');
		return result;
	} catch (err) {
		await conn.query('ROLLBACK');
		throw err;
	} finally {
		conn.release?.();
	}
}

// Batch create vertices - efficiently creates multiple nodes in a single transaction
export async function batchCreateVertices(label: string, nodesList: Array<Record<string, unknown>>) {
	const c = getAgeClient();

	return await withTransaction(async (conn) => {
		const results = [];

		for (const props of nodesList) {
			const map = propsToCypherMap(props);
			const cypher = `CREATE (n:${label} ${map}) RETURN n`;
			const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (n agtype);`;
			const result = await conn.query(sql);
			results.push(result);
		}

		return results;
	});
}

// Batch upsert (MERGE) vertices - efficiently merges multiple nodes
export async function batchUpsertVertices(
	label: string,
	nodesList: Array<{ match: Record<string, unknown>; set?: Record<string, unknown> }>,
) {
	const c = getAgeClient();

	return await withTransaction(async (conn) => {
		const results = [];

		for (const { match, set } of nodesList) {
			const matchMap = propsToCypherMap(match);
			const cypher =
				set && Object.keys(set).length > 0 ?
					`MERGE (n:${label} ${matchMap}) SET n += ${propsToCypherMap(set)} RETURN n`
				:	`MERGE (n:${label} ${matchMap}) RETURN n`;

			const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (n agtype);`;
			const result = await conn.query(sql);
			results.push(result);
		}

		return results;
	});
}

// Batch create edges with properties
export async function batchCreateEdges(
	edgeType: string,
	edgesList: Array<{
		fromLabel: string;
		fromMatch: Record<string, unknown>;
		toLabel: string;
		toMatch: Record<string, unknown>;
		properties?: Record<string, unknown>;
	}>,
) {
	const c = getAgeClient();

	return await withTransaction(async (conn) => {
		const results = [];

		for (const { fromLabel, fromMatch, toLabel, toMatch, properties } of edgesList) {
			const fromMatchMap = propsToCypherMap(fromMatch);
			const toMatchMap = propsToCypherMap(toMatch);
			const propsMap = properties && Object.keys(properties).length > 0 ? propsToCypherMap(properties) : '';

			const cypher = `
				MATCH (a:${fromLabel} ${fromMatchMap})
				MATCH (b:${toLabel} ${toMatchMap})
				MERGE (a)-[r:${edgeType} ${propsMap}]->(b)
				RETURN a, r, b
			`;

			const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (a agtype, r agtype, b agtype);`;
			const result = await conn.query(sql);
			results.push(result);
		}

		return results;
	});
}

// Execute raw Cypher query (for advanced use cases)
export async function executeCypher(cypherQuery: string) {
	const c = getAgeClient();
	const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypherQuery} $$) as (result agtype);`;
	return await c.connection.query(sql);
}

// Execute multiple Cypher queries in a single transaction
export async function executeCypherBatch(cypherQueries: string[]) {
	const c = getAgeClient();

	return await withTransaction(async (conn) => {
		const results = [];

		for (const cypher of cypherQueries) {
			const sql = `SELECT * FROM cypher('${c.graph}', $$ ${cypher} $$) as (result agtype);`;
			const result = await conn.query(sql);
			results.push(result);
		}

		return results;
	});
}

export async function disconnectAgeClient() {
	if (client) {
		try {
			if (client.connection) client.connection.release?.();
			if (client.connectionManager && typeof client.connectionManager.closeAll === 'function') {
				await client.connectionManager.closeAll();
			}
		} catch (err) {
			// ignore
		}
	}
	client = null;
}

export default {
	initAgeClient,
	getAgeClient,
	createVertex,
	findAll,
	createIp,
	createRoute,
	createDump,
	createField,
	createFieldValue,
	linkFieldToNpi,
	linkFieldValueToNpi,
	linkFields,
	withTransaction,
	batchCreateVertices,
	batchUpsertVertices,
	batchCreateEdges,
	executeCypher,
	executeCypherBatch,
	disconnectAgeClient,
};
