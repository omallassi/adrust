Feature: Decide an ADR

    Scenario: Decide an ADR
        Given An existing In Progress Decision
        When I change its status to decided
        Then The content of the file is updated to Decided

    Scenario: Decide an already decided ADR
        Given An existing not In Progress Decision
        When I update its status to decided
        Then The content of the file is not changed
