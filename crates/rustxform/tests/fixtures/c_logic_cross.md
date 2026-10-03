| survey |
|  | type | name | label | relevant | constraint | calculation |
|  | integer | age | Age |  | . >= 0 |  |
|  | begin_group | gg | GG |  |  |  |
|  | text | nm | Nm | ${age} >= 18 |  |  |
|  | end_group |  |  |  |  |  |
|  | integer | dbl | Dbl |  |  | ${age} * 2 |
| settings |
|  | form_title | form_id |
|  | F | f |
