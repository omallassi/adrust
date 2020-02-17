Feature: Manage tags on ADR

    @tags_management @regression
    Scenario: List Tags of a given decision
        Given the decision <decision_name>
        When I list all the tags
        Then I got <tag_1>, <tag_2>, <tag_3> tags

        Examples:
        | decision_name                             | tag_1       | tag_2       | tag_3       |
        | ./tests/data/my-tagged-decion-1.adoc      | tag1        | tag2        | tag3        |
        | ./tests/data/my-tagged-decion-2.adoc      | tag1        |             |             |

