export interface OrderProps {
    currentPrice: number,
    order: Order,
    onUpdate: (newOrder: Order) => void;
}

export interface Order {
    orderType: "CALL" | "PUT",
    quantity: number,
    optionId: string,
    stopLoss: number,
    targetProfit: number,
}

export default function Order({ currentPrice, order, onUpdate }: OrderProps) {
    return (
        <article>
            <form method="post" action="#">
                <fieldset className="order-form">
                    <legend>{order.optionId}</legend>
                    <div>
                        <label htmlFor="quantity">Quantity</label>
                        <input
                            type="number"
                            name="quantity"
                            step={1}
                            value={order.quantity}
                            onChange={e => onUpdate({ ...order, quantity: parseInt(e.target.value) })}
                        />
                    </div>
                    <div>
                        <label htmlFor="stopLoss">Stop Loss:</label>
                        <input
                            type="number"
                            name="stopLoss"
                            step={0.01}
                            value={order.stopLoss.toFixed(2)}
                            onChange={e => onUpdate({ ...order, stopLoss: parseFloat(e.target.value) })}
                        />
                    </div>
                    <div>
                        <label htmlFor="currentPrice">Current Price:</label>
                        <input
                            type="number"
                            name="currentPrice"
                            value={currentPrice.toFixed(2)}
                            readOnly
                            disabled
                        />
                    </div>
                    <div>
                        <label htmlFor="targetProfit">Target Profit:</label>
                        <input
                            type="number"
                            name="targetProfit"
                            step={0.01}
                            value={order.targetProfit.toFixed(2)}
                            onChange={e => onUpdate({ ...order, targetProfit: parseFloat(e.target.value) })}
                        />
                    </div>
                    <input type="submit" value="Place Order" disabled />
                </fieldset>
            </form>
        </article>
    );
}
