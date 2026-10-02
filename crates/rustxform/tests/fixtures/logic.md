| survey   |              |            |             |            |          |             |
|          | type         | name       | label       | relevant   | constraint | required  | calculation |
|          | integer      | age        | Age         |            | . >= 0     | yes       |             |
|          | text         | adult_name | Adult name  | ${age} >= 18 |          |           |             |
|          | integer      | double_age | Double age  |            |            |           | ${age} * 2  |
|          | begin_repeat | people     | People      |            |            |           |             |
|          | text         | pname      | Person name |            |            |           |             |
|          | integer      | page       | Person age  |            |            |           |             |
|          | note         | greeting   | Greeting    | ${page} > 0 |          |           |             |
|          | end_repeat   |            |             |            |            |           |             |
|          | note         | summary    | Summary     | ${double_age} > 10 |    |           |             |
| settings |            |         |
|          | form_title | form_id |
|          | Logic      | logic   |
