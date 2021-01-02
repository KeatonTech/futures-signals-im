use futures::channel::mpsc;
use std::ops::DerefMut;

// The senders vec will be cleaned up once at least this many senders
// have been closed.
pub(crate) static CHANNEL_CLEANUP_MIN_COUNT: i16 = 20;

/// Distributes an event out to each channel in a vector, removing channels
/// as they become closed.
#[inline]
pub(crate) fn notify_senders<T: Clone, D>(event: T, mut senders: D)
where
    D: DerefMut<Target = Vec<Option<mpsc::UnboundedSender<T>>>>,
{
    // Tracks how many streams in the senders vec have expired. If this
    // exceeds a certain threshold, clean up the senders vec to improve
    // performance and reduce memory use.
    let mut expired_count = 0;

    for maybe_sender in senders.iter_mut() {
        if let Some(sender) = maybe_sender {
            if sender.is_closed() {
                maybe_sender.take();
                expired_count += 1;
            } else {
                sender.unbounded_send(event.clone()).unwrap();
            }
        } else {
            expired_count += 1;
        }
    }

    if expired_count > CHANNEL_CLEANUP_MIN_COUNT {
        cleanup_expired_channels(senders);
    }
}

#[inline]
fn cleanup_expired_channels<T, D>(mut senders: D)
where
    D: DerefMut<Target = Vec<Option<mpsc::UnboundedSender<T>>>>,
{
    senders.retain(|maybe_sender| maybe_sender.is_some());
}

/// Closes all channels in a vector.
#[inline]
pub(crate) fn close_senders<T, D>(mut senders: D)
where
    D: DerefMut<Target = Vec<Option<mpsc::UnboundedSender<T>>>>,
{
    for maybe_sender in senders.iter_mut() {
        if let Some(sender) = maybe_sender {
            sender.close_channel();
        }
    }

    senders.clear();
}
