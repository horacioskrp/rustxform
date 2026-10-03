| survey   |       |          |                   |                     |                    |          |                  |                     |
|          | type  | name     | label             | constraint          | constraint_message | required | required_message | parameters          |
|          | text  | fullname | Your name         | string-length(.) > 2 | Too short         | yes      | Required!        |                     |
|          | note  | greeting | Hello ${fullname} |                     |                    |          |                  |                     |
|          | range | rating   | Rate it           |                     |                    |          |                  | start=1 end=5 step=1 |
| settings |            |          |
|          | form_title | form_id  |
|          | Advanced   | advanced |
