# Gated Vault + Transfer hook

[Turbine Task Description](https://docs.google.com/presentation/d/1cCr9ZMX3s3w5XUtAoI5U3ylHBcT7U_k1ge9a43dqMmM/edit?usp=sharing)

[![Turbine Task Description](https://drive.google.com/thumbnail?id=1cCr9ZMX3s3w5XUtAoI5U3ylHBcT7U_k1ge9a43dqMmM)](https://docs.google.com/presentation/d/1cCr9ZMX3s3w5XUtAoI5U3ylHBcT7U_k1ge9a43dqMmM/view?usp=sharing)

> 📎 Click the image to view the full presentation

<!-- <iframe
    src="https://docs.google.com/presentation/d/1cCr9ZMX3s3w5XUtAoI5U3ylHBcT7U_k1ge9a43dqMmM/embed?start=false&loop=false&delayms=3000"
    frameborder="0"
    width="960"
    height="569"
    allowfullscreen="true"
    mozallowfullscreen="true"
    webkitallowfullscreen="true">
</iframe> -->

---

## The main goal of the challenge is to **create a Transfer Hook Vault**.

### Key Requirements

- The vault must be a **single vault** where **only whitelisted users** can interact.
- The solution should handle both **depositing and removing funds**.
- The token must be **minted within the program**.
- Initial implementation should use a `Vec` for the whitelist (Pubkey and amount),
  - with a note to try different solutions like a **PDA** later.
- You are required to **test everything with LiteSVM**.
- You need to **add another extension of your choice**.
