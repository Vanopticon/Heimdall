# System Architecture Patterns

## N-Tier

An architectural pattern where functionality is separated into distinct layers, each handling specific aspects of the application. This separation provides better maintainability, scalability, and security by isolating concerns and allowing each layer to evolve independently.

```mermaid
graph TD
    subgraph "Presentation Layer"
        ui[/"User Interface"/]
    end
    subgraph "Business Layer"
        logic["Business Logic"]
    end
    subgraph "Service Layer"
        service["Services"]
    end
    subgraph "Data Layer"
        storage[(Database)]
    end
    
    ui <--"User Input/Display"--> logic
    logic <--"Business Operations"--> service
    service <--"Data Operations"--> storage
    
    style ui fill:#1a365d,stroke:#000000,color:#ffffff
    style logic fill:#153e75,stroke:#000000,color:#ffffff
    style service fill:#1e4e8c,stroke:#000000,color:#ffffff
    style storage fill:#2a4365,stroke:#000000,color:#ffffff
```

## Event Driven

A pattern where components communicate through events, enabling loose coupling and high scalability. Components can publish and subscribe to events without direct knowledge of each other, making the system highly flexible and adaptable to change.

```mermaid
graph LR
    subgraph "Event Sources"
        ui[/"UI"/]
        logic["Business Logic"]
        service["Services"]
    end
    
    subgraph "Event Bus"
        queue{"Message Queue"}
    end
    
    subgraph "Event Handlers"
        handler1["Handler 1"]
        handler2["Handler 2"]
    end
    
    ui --"Publish"--> queue
    logic --"Publish"--> queue
    service --"Publish"--> queue
    queue --"Subscribe"--> handler1
    queue --"Subscribe"--> handler2
    
    style ui fill:#1a365d,stroke:#000000,color:#ffffff
    style queue fill:#153e75,stroke:#000000,color:#ffffff
    style handler1 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style handler2 fill:#1e4e8c,stroke:#000000,color:#ffffff
```

## Microkernel Architecture

A pattern that separates core system functionality from optional plug-in components. The microkernel contains minimal functionality needed to operate the system, while additional features are implemented as plugins, enabling easy system extension and modification.

```mermaid
graph TD
    subgraph "Core System"
        kernel["Microkernel"]
        core["Core Services"]
    end
    
    subgraph "Plugin Modules"
        module1["Plugin 1"]
        module2["Plugin 2"]
        module3["Plugin 3"]
    end

    ui[/"User Interface"/] <---> kernel
    kernel <---> core
    kernel <---> module1
    kernel <---> module2
    kernel <---> module3

    style ui fill:#1a365d,stroke:#000000,color:#ffffff
    style kernel fill:#153e75,stroke:#000000,color:#ffffff
    style core fill:#2a4365,stroke:#000000,color:#ffffff
    style module1 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style module2 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style module3 fill:#1e4e8c,stroke:#000000,color:#ffffff
```

## Microservice Architecture

A distributed architectural style where the application is built as a collection of small, independent services that communicate via APIs. Each service is self-contained, independently deployable, and responsible for specific business capabilities.

```mermaid
graph TD
    client[/"Client"/]
    gateway["API Gateway"]
    auth["Auth Service"]
    svc1["Service 1"]
    svc2["Service 2"]
    svc3["Service 3"]
    db1[(Database 1)]
    db2[(Database 2)]
    db3[(Database 3)]

    client --> gateway
    gateway --> auth
    gateway --> svc1
    gateway --> svc2
    gateway --> svc3
    svc1 --> db1
    svc2 --> db2
    svc3 --> db3

    style client fill:#1a365d,stroke:#000000,color:#ffffff
    style gateway fill:#153e75,stroke:#000000,color:#ffffff
    style auth fill:#1e4e8c,stroke:#000000,color:#ffffff
    style svc1 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style svc2 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style svc3 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style db1 fill:#2a4365,stroke:#000000,color:#ffffff
    style db2 fill:#2a4365,stroke:#000000,color:#ffffff
    style db3 fill:#2a4365,stroke:#000000,color:#ffffff
```

## Shared Storage Architecture

A pattern where multiple application modules share common data storage resources. This approach enables data consistency and reduces redundancy, though careful consideration must be given to concurrency and data access patterns.

```mermaid
graph TD
    subgraph "Application Modules"
        mod1["Module 1"]
        mod2["Module 2"]
        mod3["Module 3"]
    end
    
    subgraph "Shared Resources"
        db[(Shared Database)]
        fs["File Storage"]
        cache["Cache"]
    end

    mod1 <--> db
    mod1 <--> fs
    mod1 <--> cache
    mod2 <--> db
    mod2 <--> fs
    mod2 <--> cache
    mod3 <--> db
    mod3 <--> fs
    mod3 <--> cache

    style mod1 fill:#1a365d,stroke:#000000,color:#ffffff
    style mod2 fill:#1a365d,stroke:#000000,color:#ffffff
    style mod3 fill:#1a365d,stroke:#000000,color:#ffffff
    style db fill:#2a4365,stroke:#000000,color:#ffffff
    style fs fill:#2a4365,stroke:#000000,color:#ffffff
    style cache fill:#2a4365,stroke:#000000,color:#ffffff
```

## Primary-Secondary Architecture

A fault-tolerant pattern where one node (primary) handles all write operations while multiple secondary nodes maintain synchronized copies of data. This provides high availability and read scalability while ensuring data consistency.

```mermaid
graph TD
    client[/"Client"/]
    lb["Load Balancer"]
    primary["Primary Node"]
    sec1["Secondary 1"]
    sec2["Secondary 2"]
    sec3["Secondary 3"]

    client --> lb
    lb --> primary
    primary -- "Replication" --> sec1
    primary -- "Replication" --> sec2
    primary -- "Replication" --> sec3

    style client fill:#1a365d,stroke:#000000,color:#ffffff
    style lb fill:#153e75,stroke:#000000,color:#ffffff
    style primary fill:#1e4e8c,stroke:#000000,color:#ffffff
    style sec1 fill:#2a4365,stroke:#000000,color:#ffffff
    style sec2 fill:#2a4365,stroke:#000000,color:#ffffff
    style sec3 fill:#2a4365,stroke:#000000,color:#ffffff
```

## Pipe-Filter Architecture

A pattern that processes data through a series of independent components (filters) connected by pipes. Each filter performs a specific transformation on the data and passes it to the next filter, enabling modular data processing workflows.

```mermaid
graph LR
    input[/"Input"/] --> f1["Filter 1"]
    f1 --> |"pipe"| f2["Filter 2"]
    f2 --> |"pipe"| f3["Filter 3"]
    f3 --> output[/"Output"/]

    style input fill:#1a365d,stroke:#000000,color:#ffffff
    style f1 fill:#153e75,stroke:#000000,color:#ffffff
    style f2 fill:#153e75,stroke:#000000,color:#ffffff
    style f3 fill:#153e75,stroke:#000000,color:#ffffff
    style output fill:#2a4365,stroke:#000000,color:#ffffff
```

## Message Broker

An architectural pattern that uses an intermediary component to manage and coordinate communication between distributed system components. It enables asynchronous communication and decouples message producers from consumers.

```mermaid
graph TD
    subgraph "Publishers"
        p1["Producer 1"]
        p2["Producer 2"]
    end

    subgraph "Message Broker"
        mb["Message Queue"]
        tp1["Topic 1"]
        tp2["Topic 2"]
    end

    subgraph "Subscribers"
        s1["Consumer 1"]
        s2["Consumer 2"]
        s3["Consumer 3"]
    end

    p1 --> tp1
    p2 --> tp2
    tp1 --> mb
    tp2 --> mb
    mb --> s1
    mb --> s2
    mb --> s3

    style p1 fill:#1a365d,stroke:#000000,color:#ffffff
    style p2 fill:#1a365d,stroke:#000000,color:#ffffff
    style mb fill:#153e75,stroke:#000000,color:#ffffff
    style tp1 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style tp2 fill:#1e4e8c,stroke:#000000,color:#ffffff
    style s1 fill:#2a4365,stroke:#000000,color:#ffffff
    style s2 fill:#2a4365,stroke:#000000,color:#ffffff
    style s3 fill:#2a4365,stroke:#000000,color:#ffffff
```

## Peer-to-Peer

A decentralized architecture where nodes (peers) share resources and workload without central coordination. Each peer can act as both client and server, providing high scalability and resilience to failure.

```mermaid
graph TD
    subgraph "P2P Network"
        p1["Peer 1"]
        p2["Peer 2"]
        p3["Peer 3"]
        p4["Peer 4"]
    end

    p1 <--> p2
    p2 <--> p3
    p3 <--> p4
    p4 <--> p1
    p1 <--> p3
    p2 <--> p4

    style p1 fill:#1a365d,stroke:#000000,color:#ffffff
    style p2 fill:#1a365d,stroke:#000000,color:#ffffff
    style p3 fill:#1a365d,stroke:#000000,color:#ffffff
    style p4 fill:#1a365d,stroke:#000000,color:#ffffff
```

## Layered Architecture

A classic pattern where the system is organized into layers, each with specific responsibilities. Each layer only interacts with the layer directly below or above it, promoting separation of concerns and maintainability.

```mermaid
graph TD
    presentation["Presentation Layer"]
    application["Application Layer"]
    domain["Domain Layer"]
    infrastructure["Infrastructure Layer"]

    presentation --> application
    application --> domain
    domain --> infrastructure

    style presentation fill:#1a365d,stroke:#000000,color:#ffffff
    style application fill:#153e75,stroke:#000000,color:#ffffff
    style domain fill:#1e4e8c,stroke:#000000,color:#ffffff
    style infrastructure fill:#2a4365,stroke:#000000,color:#ffffff
```

## Space-Based Architecture

A pattern designed for scalability and resilience, where processing units share access to a distributed memory space. This enables dynamic scaling and high availability, especially for systems with variable load.

```mermaid
graph TD
    client[/"Client"/]
    pu1["Processing Unit 1"]
    pu2["Processing Unit 2"]
    pu3["Processing Unit 3"]
    space["Distributed Memory Space"]

    client --> pu1
    client --> pu2
    client --> pu3
    pu1 <--> space
    pu2 <--> space
    pu3 <--> space

    style client fill:#1a365d,stroke:#000000,color:#ffffff
    style pu1 fill:#153e75,stroke:#000000,color:#ffffff
    style pu2 fill:#153e75,stroke:#000000,color:#ffffff
    style pu3 fill:#153e75,stroke:#000000,color:#ffffff
    style space fill:#2a4365,stroke:#000000,color:#ffffff
```

## Broker Architecture

A pattern where a broker component coordinates communication between clients and servers, enabling location transparency and scalability. Common in distributed systems and middleware.

```mermaid
graph TD
    client1[/"Client 1"/]
    client2[/"Client 2"/]
    broker["Broker"]
    server1["Server 1"]
    server2["Server 2"]

    client1 --> broker
    client2 --> broker
    broker --> server1
    broker --> server2

    style client1 fill:#1a365d,stroke:#000000,color:#ffffff
    style client2 fill:#1a365d,stroke:#000000,color:#ffffff
    style broker fill:#153e75,stroke:#000000,color:#ffffff
    style server1 fill:#2a4365,stroke:#000000,color:#ffffff
    style server2 fill:#2a4365,stroke:#000000,color:#ffffff
```
