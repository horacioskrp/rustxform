| survey   |                   |       |            |                |
|          | type              | name  | label      | choice_filter  |
|          | select_one states | state | State      |                |
|          | select_one cities | city  | City       | state=${state} |
| choices  |           |        |            |       |
|          | list_name | name   | label      | state |
|          | states    | tx     | Texas      |       |
|          | states    | ca     | California |       |
|          | cities    | austin | Austin     | tx    |
|          | cities    | dallas | Dallas     | tx    |
|          | cities    | la     | LA         | ca    |
| settings |            |         |
|          | form_title | form_id |
|          | Cascade    | cascade |
