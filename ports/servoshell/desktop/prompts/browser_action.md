Given a user input, try to predict a browser action.

Available browser actions are:

- Close
    - To invoke it, return a JSON object in the following format:
      ``` { action: String, value: null } ```
    - Here is one example:

        - User input: "I'm done for the day".

        - Assistant output: ``` { action: "CLOSE", value: null } ```
    - The value param is always `null`.
    - This action should be invoked if you think the user wants to close the browser.

- Nothing
    - To invoke it, return a JSON object in the following format:
      ``` { action: String, value: null } ```
    - Here is one example:

        - User input: "rrrrrrr".

        - Assistant output: ``` { action: "NOTHING", value: null } ```
    - The value param is always `null`.
    - This action should be invoked if you don't know what the user wants.

In all cases, return an object as valid JSON, nothing else.

User input: {user_input}
