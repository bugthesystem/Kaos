///usr/bin/env jbang "$0" "$@" ; exit $?
//DEPS io.aeron:aeron-all:1.47.3
//JAVA_OPTIONS --add-opens java.base/jdk.internal.misc=ALL-UNNAMED
//JAVA_OPTIONS --add-opens java.base/java.nio=ALL-UNNAMED
//JAVA_OPTIONS -Dagrona.disable.bounds.checks=true

import io.aeron.*;
import io.aeron.driver.*;
import io.aeron.logbuffer.FragmentHandler;
import org.agrona.BufferUtil;
import org.agrona.concurrent.UnsafeBuffer;

import java.nio.ByteBuffer;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Aeron UDP Benchmark - Single process, localhost
 * Comparable to: cargo run -p kaos-rudp --release --example rudp_bench
 */
public class AeronUdpBench {
    
    private static final long N = 500_000;  // Same as kaos-rudp bench
    private static final String UDP_CHANNEL = "aeron:udp?endpoint=localhost:20121";
    private static final int STREAM_ID = 1001;
    
    public static void main(String[] args) throws Exception {
        System.out.println("\n╔═══════════════════════════════════════════╗");
        System.out.println("║  Aeron UDP Benchmark (localhost)          ║");
        System.out.println("╚═══════════════════════════════════════════╝");
        System.out.printf("%nConfig: %d msgs, 8 bytes%n%n", N);
        
        final AtomicBoolean running = new AtomicBoolean(true);
        final AtomicLong received = new AtomicLong(0);
        final AtomicLong sent = new AtomicLong(0);
        
        MediaDriver.Context driverContext = new MediaDriver.Context()
            .threadingMode(ThreadingMode.SHARED)
            .dirDeleteOnStart(true)
            .dirDeleteOnShutdown(true);
        
        try (MediaDriver driver = MediaDriver.launch(driverContext);
             Aeron aeron = Aeron.connect(new Aeron.Context()
                 .aeronDirectoryName(driver.aeronDirectoryName()))) {
            
            Publication pub = aeron.addPublication(UDP_CHANNEL, STREAM_ID);
            Subscription sub = aeron.addSubscription(UDP_CHANNEL, STREAM_ID);
            
            // Wait for connection
            while (!sub.isConnected() || !pub.isConnected()) {
                Thread.sleep(10);
            }
            System.out.println("Connected!");
            
            ByteBuffer bb = BufferUtil.allocateDirectAligned(8, 64);
            UnsafeBuffer buffer = new UnsafeBuffer(bb);
            
            // Receiver thread
            Thread receiver = new Thread(() -> {
                FragmentHandler handler = (buf, offset, len, header) -> {
                    received.incrementAndGet();
                };
                while (running.get() || received.get() < sent.get()) {
                    sub.poll(handler, 256);
                }
            });
            receiver.start();
            
            Thread.sleep(100);
            long start = System.nanoTime();
            
            // Send all messages
            while (sent.get() < N) {
                buffer.putLong(0, sent.get());
                if (pub.offer(buffer, 0, 8) > 0) {
                    sent.incrementAndGet();
                }
            }
            
            double sendElapsed = (System.nanoTime() - start) / 1e9;
            System.out.printf("Send complete: %d msgs in %.3fs (%.2f M/s)%n", 
                sent.get(), sendElapsed, sent.get() / sendElapsed / 1e6);
            
            // Wait for receiver
            long timeout = System.currentTimeMillis() + 5000;
            while (received.get() < sent.get() && System.currentTimeMillis() < timeout) {
                Thread.sleep(10);
            }
            
            running.set(false);
            receiver.join(1000);
            
            double totalElapsed = (System.nanoTime() - start) / 1e9;
            
            System.out.println("\n═══════════════════════════════════════════");
            System.out.printf("  Sent:      %d msgs%n", sent.get());
            System.out.printf("  Received:  %d msgs (%.1f%%)%n", 
                received.get(), 100.0 * received.get() / sent.get());
            System.out.printf("  Duration:  %.3fs%n", totalElapsed);
            System.out.printf("  Throughput: %.2f M/s%n", received.get() / totalElapsed / 1e6);
            System.out.println("═══════════════════════════════════════════\n");
        }
    }
}

