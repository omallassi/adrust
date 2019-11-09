Feature: Create an ADR

    Scenario: Create a new ADR
        Given A decision my Decision 13 I need to make
        When I create a new Decision Record
        Then A new file named my-decision-13 is created

    Scenario: Create an ADR that already exists
        Given A new decision my-decision that already exists
        When I create a new ADR
        Then The creation fails