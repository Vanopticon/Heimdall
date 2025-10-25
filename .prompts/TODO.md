# Implementation Roadmap

## Phase 1: Design

1. Create a `docs/design/Requirements.md` documenting the functional and non-functional requirements of the project. If one already exists, review it for completeness, spelling, grammar, and clarity and make refinements.
2. Create a `docs/design/TODO.md` if one does not exist. List the tasks, broken down into logical phases, required to complete the project as specified. Include the following for each task:
	- A short summary of the task.
	- Reeferences to one or more specific requirements (required).
	- References to specific design documents.
	- A breakout of implementation details.
	- A list of tests that must be included.
	- A list of examples that must be included.
	- A list of documentation that must be created or updated.
	- Any other pertinent information that the implementer may need.

## Phase 2

### Priority Features

#### Feature Name

##### Objective

<!-- What is the purpose of the feature -->

##### Implementation Details

1. Do x
2. Do y
3. Add comprehensive tests
4. Create examples
5. Create and update documentation

## Phase 3

1. **Priority Features**

    - Framework and documentation for developers to build their own components using Engage UX.

2. **Additional Features**

    - Animation system
    - Drag and drop support
    - Ability for developers to provide their own input handler for other devices.

3. **Testing**
    - Add integration tests
    - Add end-to-end functional tests
    - Platform-specific testing

## Phase 4

1. **Priority Features**

    - Support relative values for properties, e.g. `rb` and `rp`, where `rb` operates similarly to `em` in that it is a scaling relative to the theme's base size, and `rp` operates similarly to `rem` by scaling relative to the inherited size.
    - Support layout properties in the theme for components mapped to the `id` or a `name` property. Each component should be able to be positioned absolutely or relative to it's direct parent. Support `width`, `height`, `top`, `left`,`bottom`,`right`,`min_width`,`max_width`,`min_height`,`max_height` properties. Support an alternative sizing mode of `fill` that takes no sizes and fills the parent (an enum would be appropriate here). Support relative values, such as `rb`, `rp`, and `%`.

2. **Additional Features**
    - Support for multi-monitor setups, allowing devs to treat them as one pane, multiple separate panes, or a mix (for >2 monitors). Support for runtime configuration is required, as devs may want to allow users to choose.

## Phase 5

- Support for client/server rendering.
    - Mode 1 (default): The server renders the image, using the monitor layout of the client, and sends the compressed, rendered view to the client. The client then displays the view, and returns any input to the server. This is meant for use cases where the server has the rendering horsepower.
    - Mode 2: The server sends all information required to render the UX to the client who then renders the view. The client sends any input events to the server. This is meant for the use case where the client has rendering horsepower.
    - The entire connection must be encrypted with a minimum equivalent to TLS 1.3 (you may use HTTPS and TLS 1.3 if it will be performant enough). Both client and server must support using the OS Certificate Authorities as well as configurable additional CAs. The server must support both encrypted and unencrypted key files.
    - All connections must be fully authenticated. Support for built in Windows authentication (including Active Directory), Linux PAM, LDAP, and OAuth are required. Support for basic user/password authentication will not be supported.
    - Minimally, the system must be able to render, send, and display 24fps video without noticable stuttering or any degradation. Ideally, it should be able to support 4k 120fps, given sufficient bandwidth.
