package profiling.implementation;

import com.mendix.core.Core;
import com.mendix.logging.LogLevel;
import com.mendix.logging.LogMessage;
import com.mendix.logging.LogSubscriber;
import com.mendix.thirdparty.org.json.JSONObject;

public class MicroflowExecutionLogSubscriber extends LogSubscriber {
	private volatile boolean stopped = true;
	private boolean registered = false;

	public MicroflowExecutionLogSubscriber(String arg0, LogLevel arg1) {
		super(arg0, arg1);
		
	}

	public void start() {
		if (!registered) {
			Core.registerLogSubscriber(this);
			this.registered = true;
		}

		this.stopped = false;
	}
	
    public void stop()
    {
        this.stopped = true;
    }
	
	@Override
	public void processMessage(LogMessage logMessage) {
		if (!stopped) {
			if (!logMessage.node.name().equals("MicroflowEngine")) {
				return;
			}
			
			TCPClient client = TCPClient.INSTANCE;
			JSONObject jo = new JSONObject();
			jo.put("type", "LogMessage");
			jo.put("node_name", logMessage.node.name());
			jo.put("timestamp", logMessage.timestamp);
			jo.put("message", logMessage.message.toString());

			try {
				client.sendMessage(jo.toString());
			} catch (Exception e) {
				e.printStackTrace();
			}
		}
		
	}

}
