#!/usr/bin/python3
import argparse
import asyncio
import logging
import json
import sys
import csv
import matplotlib.pyplot as plt


from miniconf import Miniconf
from gmqtt import Client as MqttClient

MAXLEN = 10000


class TelemetryReader:
    """ Helper utility to read Stabilizer telemetry. """

    @classmethod
    async def create(cls, prefix, broker, queue):
        """Create a connection to the broker and an MQTT device using it."""
        client = MqttClient(client_id='')
        await client.connect(broker)
        return cls(client, prefix, queue)

    def __init__(self, client, prefix, queue):
        """ Constructor. """
        self.client = client
        self._telemetry = []
        self.client.on_message = self.handle_telemetry
        self._telemetry_topic = f'{prefix}/telemetry'
        self.client.subscribe(self._telemetry_topic)
        self.queue = queue

    def handle_telemetry(self, _client, topic, payload, _qos, _properties):
        """ Handle incoming telemetry messages over MQTT. """
        assert topic == self._telemetry_topic
        self.queue.put_nowait(json.loads(payload))


async def get_tele(telemetry_queue):
    latest_values = await telemetry_queue.get()
    return [latest_values['adcs'][0], latest_values['dacs'][0]]


def main():
    """ Main program entry point. """
    parser = argparse.ArgumentParser(description='record thermostat-mqtt data')
    parser.add_argument('--broker', '-b', type=str, default='mqtt',
                        help='The MQTT broker to use to communicate with Stabilizer')
    parser.add_argument('--prefix', '-p', type=str, required=True,
                        help='The Stabilizer device prefix to use for communication. E.g. '
                        'dt/sinara/dual-iir/00-11-22-33-44-55')
    parser.add_argument('--channel', '-c', type=int, choices=[0, 1], default=0,
                        help='The filter channel to configure.')
    parser.add_argument('--telemetry_rate', type=int, default=1,
                        help='The number of Stabilizer hardware ticks between each sample')

    args = parser.parse_args()

    print(args.prefix)

    telemetry_queue = asyncio.LifoQueue()

    async def telemetry():
        await TelemetryReader.create(args.prefix, args.broker, telemetry_queue)
        try:
            while True:
                await asyncio.sleep(1)
        except asyncio.CancelledError:
            pass

    telemetry_task = asyncio.Task(telemetry())

    async def record():
        interface = await Miniconf.create(args.prefix, args.broker)

        await interface.command('telemetry_period', 1.0, retain=False)
        await interface.command('adcsettings/odr', 18, retain=False)

        # fig = plt.figure()
        # ax = fig.add_subplot(1, 1, 1)
        # plt.show()

        f = open('adcvals.csv', 'w')
        writer = csv.writer(f)
        data = []
        for i in range(MAXLEN):
            data.append(await get_tele(telemetry_queue))
            writer.writerow([data[i][0], data[i][1]])
            print(f'temp: {data[i][0]}, curr: {data[i][1]}')
            # ax.clear()
            # ax.plot(temp)

        f.close()
        telemetry_task.cancel()

    loop = asyncio.get_event_loop()
    # loop.run_until_complete(record())
    sys.exit(loop.run_until_complete(record()))


if __name__ == '__main__':
    main()
