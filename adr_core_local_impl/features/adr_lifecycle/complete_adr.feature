Feature: Complete or Supersede or... an ADR

    Scenario: Completes an already decided ADR
        Given A Decided Decision
        When I update its status to completed-by
        Then The status of the initial decision is completed-by
        And The status of the new decision is completes
