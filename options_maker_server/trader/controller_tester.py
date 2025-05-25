# import asyncio
# import datetime
#
# import pandas as pd
# from lightweight_charts import Chart
#
# import db
# from db.instruments import Price
# from trader import Controller
# from utils.prices import agg_prices
# from utils.times import MY_TIME_ZONE
#
#
# async def plot_price_lines(ctr: Controller):
#     df = ctr._lower_time_frame.copy()
#     df.index = df.index.tz_localize(None)
#
#     def on_timeframe_selection(chart):
#         bars = agg_prices(df, chart.topbar["timeframe"].value)
#         chart.set(bars, keep_drawings=True)
#
#     chart = Chart(width=1000, height=800)
#     chart.set(df)
#     chart.topbar.textbox("symbol", ctr.symbol)
#     chart.topbar.switcher(
#         "timeframe",
#         ("1min", "5min", "15min", "30min", "60min"),
#         default="1min",
#         func=on_timeframe_selection,
#     )
#     for price_level in ctr._price_levels:
#         line = chart.create_line(color='yellow', width=min(2, int(10 * price_level.weight)), style="large_dashed",
#                                  price_line=False, price_label=False)
#         data = pd.DataFrame([
#             {"time": price_level.at.tz_localize(None), "value": price_level.price},
#             {"time": df.iloc[-1].name, "value": price_level.price},
#         ])
#         line.set(data)
#
#     await chart.show_async()
#
#
# def get_dataframe(ctr: Controller):
#     json = ctr.to_json()
#     df = pd.DataFrame(json["higher_time_frame_bars"])
#     df.set_index("time", inplace=True)
#     return df, json["divergences"]
#
#
# def split_at(prices, at):
#     at = datetime.datetime.strptime(at, "%Y-%m-%d %H:%M").astimezone(MY_TIME_ZONE)
#     old, new = [], []
#     for price in prices:
#         if price.time < at:
#             old.append(price)
#         else:
#             new.append(price)
#     return old, new
#
#
# if __name__ == "__main__":
#     async def main():
#         await db.init_db()
#         symbol = "GOOG"
#         prices = await Price.filter(symbol=symbol).order_by("time")
#         old, new = split_at(prices, "2025-05-21 7:00")
#
#         price_chart = Chart(height=800, width=1000, inner_width=1, inner_height=0.7)
#         chart2 = price_chart.create_subchart(width=1, height=0.3, sync=True)
#         rsi_line = chart2.create_line()
#
#         controller = Controller(symbol, old)
#
#         df, _ = get_dataframe(controller)
#         price_chart.set(df)
#         rsi_line.set(df.rsi.rename("value").to_frame())
#         price_chart.show()
#
#         div_lines = []
#
#         for price in new:
#             controller.on_new_price(price)
#             df, _ = get_dataframe(controller)
#             divergences = controller._divergences
#             price_chart.update(df.iloc[-1])
#             rsi_line.update(df.rsi.rename("value").to_frame().iloc[-1])
#
#             if len(divergences) > 0:
#                 for line in div_lines:
#                     line.delete()
#                     pass
#                 for div in divergences:
#                     l1 = price_chart.create_line(color="yellow", width=1, price_line=False, price_label=False)
#                     l1.set(pd.DataFrame([
#                         {"time": div.start.astimezone(MY_TIME_ZONE).tz_localize(None), "value": div.start_price},
#                         {"time": div.end.astimezone(MY_TIME_ZONE).tz_localize(None), "value": div.end_price},
#                     ]))
#                     div_lines.append(l1)
#             await asyncio.sleep(.25)
#
#         # await plot_price_lines(controller)
#
#
#     asyncio.run(main())
