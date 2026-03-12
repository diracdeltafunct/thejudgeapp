export function initDeckCounter(container: HTMLElement): void {
  const history: number[] = [];

  function render(): void {
    const total = history.reduce((sum, n) => sum + n, 0);
    const historyStr = history.length > 0 ? history.join("+") : "—";

    container.innerHTML = `
      <div class="deck-counter">
        <div class="counter-displays">
          <div class="counter-history">${historyStr}</div>
          <div class="counter-total">${total}</div>
        </div>
        <div class="counter-buttons">
          <button class="counter-btn counter-num" data-value="1">1</button>
          <button class="counter-btn counter-num" data-value="2">2</button>
          <button class="counter-btn counter-num" data-value="3">3</button>
          <button class="counter-btn counter-num" data-value="4">4</button>
          <button class="counter-btn counter-action" id="counter-undo">Undo</button>
          <button class="counter-btn counter-action counter-clear" id="counter-clear">Clear</button>
        </div>
      </div>
    `;

    container.querySelectorAll<HTMLButtonElement>(".counter-num").forEach((btn) => {
      btn.addEventListener("click", () => {
        history.push(Number(btn.dataset.value));
        render();
      });
    });

    container.querySelector("#counter-undo")!.addEventListener("click", () => {
      history.pop();
      render();
    });

    container.querySelector("#counter-clear")!.addEventListener("click", () => {
      history.length = 0;
      render();
    });
  }

  render();
}
