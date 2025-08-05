Given a user input, try to predict a browser action.

Available browser actions are:

- Navigate
    - To invoke it, return a JSON object in the following format:
      ``` { action: String, value: [String] } ```

    - Here is one example:

        - User input: "I feel like writing some code while listening to my youtube mix".

        - Assistant output: ``` { action: "NAVIGATE", value: ["https://www.youtube.com", "https://github.com"] } ```

    - The value of the action can be one or several web pages to navigate to,
      based on user wishes implied by the input, and using the below history of navigations,
      as well as you own knowledge of top global sites.

    - Navigation history: `[ { name: "github", url: "https://github.com", }, { name: "guardian", url: "https://theguardian.com", },]`

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
