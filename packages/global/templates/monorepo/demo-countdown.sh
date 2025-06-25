DURATION=${1:-3}

while [ $DURATION -gt 0 ]; do
    echo "Countdown: $DURATION seconds remaining..."
    sleep 1
    ((DURATION--))
done

exit 0
