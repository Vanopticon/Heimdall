# Heimdall Graph Data Model

This document describes Heimdall's graph-centric data model for representing and correlating security-relevant entities. It combines traditional tabular records with graph nodes and edges to model indicators of compromise (IoCs), their occurrences in data dumps, and the relationships between fields, categories, and infrastructure. The model supports deduplication of repeated values, provenance tracking across rows/fields/dumps, and expressive graph queries that surface context and linkage at scale.

```mermaid
classDiagram
    class dumps {
        +id: int
        +name: String
        +signature: u128
    }

    class sources {
        +id: int
        +name: String
    }
    class columns {
        +id: int
        +name: String
        +description: String
    }
    class rows {
        +dump_id: int
        +row_id: int
    }
    class cells {
        +row_id: int
        +column_id: int
        +value: u8[]
    }
    class sightings {
        +detected_at
    }

    sources --> dumps
    dumps --> columns
    dumps --> rows
    rows --> cells
    columns --> cells
    cells --> sightings: edge(count)

    sources --> sightings
    iocs --> sightings: edge(count)
```
