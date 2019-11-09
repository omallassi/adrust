Feature: Create an ADR

    Scenario: Create a new ADR
        Given A decision my-decision I need to make
        When I want to create a new Decision Record
        Then I can create a new ADR

    Scenario: Create an ADR that already exists
        Given A new decision my-decision that already exists
        When I create a new ADR
        Then The creation fails