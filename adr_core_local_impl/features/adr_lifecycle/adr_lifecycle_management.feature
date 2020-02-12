Feature: Manage an ADR Lifecycle

    Scenario: Create a new ADR
        Given A decision my Decision 13 I need to make
        When I create a new Decision Record
        Then A new file named my-decision-13 is created
        And the status is wip
        And the title is my decision 13

    Scenario: Create an ADR that already exists
        Given A new decision my-decision that already exists
        When I create a new ADR
        Then The creation fails

    # Check all transitions
    Scenario Outline: Check transitions and lifecycle of ADR
        Given a decision with status <status>
        When the decision is transitioned to <transition_status> by <by>
        Then the transition is <accepted>
        And the new status is <new_status>
        And the date is updated to today

        Examples:
            | status        | transition_status | accepted  | new_status    | by                |
            | wip           | decided           | true      | decided       | n/a               |
            | decided       | decided           | false     | decided       | n/a               |
            | completed-by  | decided           | false     | completed-by  | completes adr     |
            | completes     | decided           | false     | completes     | completed-by adr  |
            | superseded-by | decided           | false     | superseded-by | supersedes adr    |
            | supersedes    | decided           | false     | supersedes    | superseded-by adr |
            | cancelled     | decided           | false     | cancelled     | n/a               |


