package profiling.implementation;

import java.io.*;

import java.net.Socket;
import java.net.InetSocketAddress;
import java.nio.channels.SocketChannel;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

import com.mendix.core.Core;
import com.mendix.logging.ILogNode;
import com.mendix.logging.LogLevel;
import com.mendix.systemwideinterfaces.core.IContext;
import com.mendix.thirdparty.org.json.JSONObject;
import com.mendix.thirdparty.org.json.JSONTokener;
import profiling.proxies.FlameGraph;

public enum TCPClient {
	INSTANCE;

	private SocketChannel clientChannel;


	private DisconnectCallback disconnectCallback;
	private volatile boolean disconnected = false;  // Flag to prevent multiple callbacks

	private final MicroflowExecutionLogSubscriber subs = new MicroflowExecutionLogSubscriber("MicroflowExecutionSubscriber", LogLevel.TRACE);

	private final ExecutorService executor = Executors.newFixedThreadPool(4);
	private final ArrayBlockingQueue<String> messageQueue = new ArrayBlockingQueue<>(10000);


	TCPClient() {
		try {
			openChannel();
		} catch (IOException e) {
			e.printStackTrace();
		}
	}

	private void openChannel() throws IOException {
		if (clientChannel == null || !clientChannel.isOpen()) {
			clientChannel = SocketChannel.open();
			clientChannel.configureBlocking(false);
		}
	}

	public void connect(String ip, int port) throws IOException {
		ILogNode logger = Core.getLogger("Profiling");

		if (clientChannel != null && !clientChannel.isOpen()) {
			openChannel();
		}

		if (!isConnected()) {
			if (clientChannel != null && clientChannel.isConnected()) {
				clientChannel.close();
				openChannel();
			}
			clientChannel.connect(new InetSocketAddress(ip, port));

			while (!clientChannel.finishConnect()) {
				// Wait until the connection is completed
				Thread.yield();
			}
			logger.info("ClientChannel successfully opened.");

			disconnected = false;
			listenForDisconnect();
			startSendingMessages();
		}
		else {
			logger.info("Profiler already connected.");
		}

	}
	
	public boolean isConnected() {
		return clientChannel != null && clientChannel.isOpen() && clientChannel.isConnected() && !disconnected;
	}

	// Send a message to the server
	public void sendMessage(String msg) throws IOException {
		if (!messageQueue.offer(msg)) {
			// Handle message queue full scenario, e.g., log a warning
			ILogNode logger = Core.getLogger("Profiling");
			logger.error("Message queue full!");
		}
	}

	// Disconnect from the server

	public void disconnect() {

		if (isConnected()) {
			try {
				clientChannel.close();
				triggerDisconnectCallback();
			} catch (IOException e) {
				e.printStackTrace();
			}
		}
	}

	// Set the disconnect callback

	public void setDisconnectCallback(DisconnectCallback callback) {

		this.disconnectCallback = callback;

	}

	
    // Trigger the disconnect callback

    private void triggerDisconnectCallback() {
        if (!disconnected && disconnectCallback != null) {
        	stopLogging();
            disconnected = true;  // Set the flag to true
			disconnect();
            disconnectCallback.onDisconnect();
        }
    }
    
	// Listen for disconnection

	private void listenForDisconnect() {

		Thread messageListener = new Thread(() -> {
			ILogNode logger = Core.getLogger("Profiling");

			try {
				while (clientChannel != null && clientChannel.isConnected()) {

					ByteBuffer lengthBuffer = ByteBuffer.allocate(4);
					int bytesRead = clientChannel.read(lengthBuffer);
					if (bytesRead == 4) {
						lengthBuffer.flip();
						int responseLength = lengthBuffer.getInt();

						logger.info("Retrieved message, reading " + responseLength + " bytes");
						ByteBuffer responseBuffer = ByteBuffer.allocate(responseLength);
						while (responseBuffer.hasRemaining()) {
							int readBytes = clientChannel.read(responseBuffer);
							if (readBytes == -1) {
								triggerDisconnectCallback();
								return;
							}
						}
						responseBuffer.flip();

						byte[] responseBytes = new byte[responseLength];
						responseBuffer.get(responseBytes);

						JSONObject json = new JSONObject(new JSONTokener(new String(responseBytes, StandardCharsets.UTF_8)));

						String responseType = json.getString("type");
						logger.info("Retrieved message, of type " + responseType);

						if (responseType.equals("FileResponse")) {
							String identifier = json.getString("identifier");
							byte[] fileContent = Base64.getDecoder().decode(json.getString("content"));

							IContext context = Core.createSystemContext();
							FlameGraph graph = new FlameGraph(context);
							graph.setidentifier(identifier);

							ByteArrayInputStream istream = new ByteArrayInputStream(fileContent);
							Core.storeFileDocumentContent(context, graph.getMendixObject(), identifier + ".svg", istream);
							istream.close();

							logger.info("read and stored file: " + identifier);
						}
					}
					else if (bytesRead == -1) {
						triggerDisconnectCallback();
						return;
					}

				}
			} catch (IOException e) {
				triggerDisconnectCallback();
			}

		});

		messageListener.start();
	}

	private void startSendingMessages() {
		executor.submit(() -> {
			try {
				while (isConnected() || !messageQueue.isEmpty()) {
					String msg = messageQueue.poll(1, TimeUnit.SECONDS);
					if (msg != null && clientChannel.isOpen()) {
						ByteBuffer buffer = ByteBuffer.allocate(4 + msg.length());
						buffer.putInt(msg.length());
						buffer.put(msg.getBytes(StandardCharsets.UTF_8));
						buffer.flip();
						clientChannel.write(buffer);
					}
				}
			} catch (IOException | InterruptedException e) {
				triggerDisconnectCallback();
			}
		});
	}

	public void startLogging() {
		subs.start();
	}
	
	public void stopLogging() {
        ILogNode logger = Core.getLogger("Profiling");
        logger.info("Logger stopped!");
        subs.stop();
	}

}
