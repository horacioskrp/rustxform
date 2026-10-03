| survey   |         |      |       |         |
|          | type    | name | label | save_to |
|          | text    | sp   | Sp    | species |
|          | integer | ok   | OK    |         |
| entities |         |         |            |
|          | dataset | label   | create_if  |
|          | trees   | ${sp}   | ${ok} > 0  |
| settings |            |         |
|          | form_title | form_id |
|          | CI         | ci      |
