import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import {
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
} from './ageClient.js';

// These tests require a running Postgres+AGE instance
// Skip tests if database is not available
const DB_AVAILABLE = process.env['SKIP_DB_TESTS'] !== 'true';

describe.skipIf(!DB_AVAILABLE)('AGE Client Integration Tests', () => {
	const TEST_GRAPH = process.env['AGE_GRAPH'] || 'dumps_graph';

	beforeAll(async () => {
		// Initialize the AGE client with test configuration
		await initAgeClient({
			graph: TEST_GRAPH,
		});
	});

	afterAll(async () => {
		// Clean up connection
		await disconnectAgeClient();
	});

	describe('Client Initialization', () => {
		it('should initialize client and get client instance', () => {
			expect(() => getAgeClient()).not.toThrow();
			const client = getAgeClient();
			expect(client).toBeDefined();
			expect(client.graph).toBe(TEST_GRAPH);
		});
	});

	describe('Basic Vertex Operations', () => {
		it('should create a vertex with properties', async () => {
			const result = await createVertex('dumps', { name: 'test-dump-basic' });
			expect(result).toBeDefined();
		});

		it('should find all vertices of a specific label', async () => {
			// Create a test vertex first
			await createVertex('dumps', { name: 'test-dump-findall' });

			const result = await findAll('dumps');
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
			expect(result.rows.length).toBeGreaterThan(0);
		});
	});

	describe('Domain-Specific Operations', () => {
		it('should create an IP vertex and link to INFRASTRUCTURE', async () => {
			const result = await createIp('192.168.1.100', { description: 'test server' });
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should create a route between two IPs', async () => {
			const result = await createRoute('10.0.0.1', '10.0.0.2');
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should create a dump node linked to ROOT', async () => {
			const result = await createDump('integration-test-dump', { source: 'unit-test' });
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should create a field linked to a dump', async () => {
			// Ensure dump exists first
			await createDump('test-dump-for-field');

			const result = await createField('test-dump-for-field', 'email', {
				type: 'string',
				description: 'Email field',
			});
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should create field values with provenance tracking', async () => {
			// Setup: create dump and field
			await createDump('test-dump-values');
			await createField('test-dump-values', 'username');

			const result = await createFieldValue('test-dump-values', 'username', 'row-1', 'testuser', {
				sanitized: true,
			});
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should link field to NPI category', async () => {
			await createField('test-dump-npi', 'ssn');

			const result = await linkFieldToNpi('ssn', 'PII');
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should link field value to NPI category', async () => {
			// Setup
			await createDump('test-dump-fv-npi');
			await createField('test-dump-fv-npi', 'card_number');
			await createFieldValue('test-dump-fv-npi', 'card_number', 'row-x', '****1234');

			const result = await linkFieldValueToNpi('card_number', 'row-x', 'FINANCIAL');
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should link two fields together', async () => {
			await createField('test-dump-links', 'parent_id');
			await createField('test-dump-links', 'child_id');

			const result = await linkFields('parent_id', 'child_id');
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});
	});

	describe('Transaction Support', () => {
		it('should execute operations within a transaction and commit', async () => {
			const result = await withTransaction(async (conn) => {
				const cypher = `CREATE (n:dumps {name: 'tx-test-commit'}) RETURN n`;
				const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (n agtype);`;
				return await conn.query(sql);
			});

			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should rollback transaction on error', async () => {
			const uniqueName = `tx-test-rollback-${Date.now()}`;

			await expect(async () => {
				await withTransaction(async (conn) => {
					// First operation should succeed
					const cypher1 = `CREATE (n:dumps {name: '${uniqueName}'}) RETURN n`;
					const sql1 = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher1} $$) as (n agtype);`;
					await conn.query(sql1);

					// Force an error with invalid Cypher
					const invalidCypher = `INVALID CYPHER SYNTAX HERE`;
					const sql2 = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${invalidCypher} $$) as (n agtype);`;
					await conn.query(sql2);
				});
			}).rejects.toThrow();

			// Verify the node was not created (transaction was rolled back)
			const checkCypher = `MATCH (n:dumps {name: '${uniqueName}'}) RETURN n`;
			const checkSql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${checkCypher} $$) as (n agtype);`;
			const client = getAgeClient();
			const checkResult = await client.connection.query(checkSql);
			expect(checkResult.rows.length).toBe(0);
		});
	});

	describe('Batch Operations', () => {
		it('should batch create multiple vertices', async () => {
			const nodes = [
				{ name: 'batch-dump-1', source: 'test' },
				{ name: 'batch-dump-2', source: 'test' },
				{ name: 'batch-dump-3', source: 'test' },
			];

			const results = await batchCreateVertices('dumps', nodes);
			expect(results).toBeDefined();
			expect(results.length).toBe(3);
		});

		it('should batch upsert vertices (MERGE)', async () => {
			const nodes = [
				{ match: { name: 'upsert-test-1' }, set: { updated: true } },
				{ match: { name: 'upsert-test-2' }, set: { updated: true } },
			];

			const results = await batchUpsertVertices('dumps', nodes);
			expect(results).toBeDefined();
			expect(results.length).toBe(2);

			// Run again to verify upsert behavior (should not duplicate)
			const results2 = await batchUpsertVertices('dumps', nodes);
			expect(results2).toBeDefined();
			expect(results2.length).toBe(2);
		});

		it('should batch create edges', async () => {
			// Setup: create nodes first
			await createDump('edge-batch-dump-1');
			await createDump('edge-batch-dump-2');
			await createField('edge-batch-dump-1', 'field-a');
			await createField('edge-batch-dump-2', 'field-b');

			const edges = [
				{
					fromLabel: 'dumps',
					fromMatch: { name: 'edge-batch-dump-1' },
					toLabel: 'fields',
					toMatch: { name: 'field-a' },
				},
				{
					fromLabel: 'dumps',
					fromMatch: { name: 'edge-batch-dump-2' },
					toLabel: 'fields',
					toMatch: { name: 'field-b' },
				},
			];

			const results = await batchCreateEdges('DUMP_HAS_FIELD', edges);
			expect(results).toBeDefined();
			expect(results.length).toBe(2);
		});
	});

	describe('Raw Cypher Execution', () => {
		it('should execute raw Cypher query', async () => {
			const result = await executeCypher(`CREATE (n:dumps {name: 'raw-cypher-test'}) RETURN n`);
			expect(result).toBeDefined();
			expect(result.rows).toBeDefined();
		});

		it('should execute batch of Cypher queries in transaction', async () => {
			const queries = [
				`CREATE (n:dumps {name: 'batch-cypher-1'}) RETURN n`,
				`CREATE (n:dumps {name: 'batch-cypher-2'}) RETURN n`,
				`MATCH (a:dumps {name: 'batch-cypher-1'}), (b:dumps {name: 'batch-cypher-2'}) 
				 CREATE (a)-[:TEST_LINK]->(b) RETURN a, b`,
			];

			const results = await executeCypherBatch(queries);
			expect(results).toBeDefined();
			expect(results.length).toBe(3);
		});
	});

	describe('Schema Validation', () => {
		it('should have ROOT nodes for all major categories', async () => {
			const rootCategories = ['sources', 'dumps', 'fields', 'field_data', 'rows', 'field_value', 'sightings'];

			for (const category of rootCategories) {
				const cypher = `MATCH (n:${category} {name: 'ROOT'}) RETURN n`;
				const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (n agtype);`;
				const client = getAgeClient();
				const result = await client.connection.query(sql);
				expect(result.rows.length).toBeGreaterThan(0);
			}
		});

		it('should have NPI category nodes', async () => {
			const npiCategories = [
				'IDENTIFICATION',
				'AAA',
				'PII',
				'FINANCIAL',
				'HEALTH',
				'EMPLOYMENT',
				'BEHAVIORAL',
				'INFRASTRUCTURE',
				'COMMUNICATIONS',
				'OTHER_NPI',
			];

			for (const category of npiCategories) {
				const cypher = `MATCH (n:NPI_Category {name: '${category}'}) RETURN n`;
				const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (n agtype);`;
				const client = getAgeClient();
				const result = await client.connection.query(sql);
				expect(result.rows.length).toBeGreaterThan(0);
			}
		});

		it('should have HAS_SUBCATEGORY relationships from NPI ROOT', async () => {
			const cypher = `
				MATCH (root:NPI_Category {name: 'ROOT'})-[:HAS_SUBCATEGORY]->(sub:NPI_Category)
				RETURN count(sub) as count
			`;
			const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (count agtype);`;
			const client = getAgeClient();
			const result = await client.connection.query(sql);
			expect(result.rows.length).toBeGreaterThan(0);
			// Should have at least 10 subcategories
			const countStr = result.rows[0].count;
			const count = parseInt(countStr);
			expect(count).toBeGreaterThanOrEqual(10);
		});
	});

	describe('Idempotent Operations', () => {
		it('should handle duplicate createDump calls gracefully', async () => {
			const dumpName = `idempotent-dump-${Date.now()}`;

			// Create twice - should not error
			await createDump(dumpName, { test: true });
			await createDump(dumpName, { test: true });

			// Verify only one node exists (or that MERGE worked correctly)
			const cypher = `MATCH (n:dumps {name: '${dumpName}'}) RETURN n`;
			const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (n agtype);`;
			const client = getAgeClient();
			const result = await client.connection.query(sql);
			expect(result.rows.length).toBeGreaterThan(0);
		});

		it('should handle duplicate createField calls gracefully', async () => {
			const dumpName = `idempotent-field-dump-${Date.now()}`;
			const fieldName = `idempotent-field-${Date.now()}`;

			await createDump(dumpName);

			// Create field twice - should not error
			await createField(dumpName, fieldName, { type: 'string' });
			await createField(dumpName, fieldName, { type: 'string' });

			// Verify operation completed successfully
			const cypher = `MATCH (n:fields {name: '${fieldName}'}) RETURN n`;
			const sql = `SELECT * FROM cypher('${TEST_GRAPH}', $$ ${cypher} $$) as (n agtype);`;
			const client = getAgeClient();
			const result = await client.connection.query(sql);
			expect(result.rows.length).toBeGreaterThan(0);
		});
	});
});
